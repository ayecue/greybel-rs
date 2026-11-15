//! Compiling and installing programs for a script's shell (`ShellIntrinsics.build`,
//! `PlayerUtils.ImportCode`).

use greybel_source_map::SourceMap;
use greyhack_behaviour::{
    FileData, FileKind, Import, MAX_IMPORTED_CODE, Permission, PermissionSet, import_paths, path,
    put_imports,
};
use greyhack_world_core::{ComputerId, NodeDraft, Result};

/// Why `import_code` does not produce a file (`PlayerUtils.ImportCode`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportFailure {
    /// The path is not absolute.
    RelativePath(String),
    /// No file is at the path.
    MissingFile(String),
    /// The file is imported already.
    DuplicateImport(String),
    /// The code with the imports is over the game's size limit.
    TooLarge,
    /// The file is binary, not a program.
    BinaryFile(String),
    /// The program does not allow being imported.
    NoImportPermission(String),
    /// The user cannot read the file.
    Unreadable(String),
}

/// Why a build or a binary write fails (`ShellIntrinsics.build`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildFailure {
    /// The shell's computer no longer exists.
    ComputerUnavailable,
    /// No source file is at the path.
    MissingSource(String),
    /// No folder is at the path.
    MissingFolder(String),
    /// The user cannot read the source.
    UnreadableSource(String),
    /// The named source file is binary.
    BinarySource(String),
    /// The user cannot write into the folder.
    UnwritableFolder(String),
    /// The named source file has no extension to remove.
    InvalidExtension(String),
    /// The source is empty.
    EmptySource,
    /// An import fails.
    Import(ImportFailure),
    /// An `import_code(` has no closing parenthesis, which crashes the game.
    UnclosedImport,
}

/// Source of a build with the name of its binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltSource {
    /// Source with its imports expanded.
    pub source: String,
    /// Name of the binary.
    pub name: String,
    /// Source map of the expanded code.
    pub source_map: SourceMap,
}

/// Source builds, imports, binary writes, and shell copies.
impl BuildService {
    pub async fn build_source(
        &self,
        (computer, user): (ComputerId, &str),
        (session_computer, session_user): (ComputerId, &str),
        source: &str,
        folder: &str,
    ) -> Result<std::result::Result<BuiltSource, BuildFailure>> {
        if self.networks.topology.computer(computer).await?.is_none() {
            return Ok(Err(BuildFailure::ComputerUnavailable));
        }

        let (code, name) = match self.source_code(computer, user, source, folder).await? {
            Ok(code) => code,
            Err(failure) => return Ok(Err(failure)),
        };
        let session_computer = self
            .networks
            .topology
            .computer(session_computer)
            .await?
            .map(|computer| computer.id);

        Ok(
            match self
                .expand_imports(session_computer, session_user, code)
                .await?
            {
                Ok((source, source_map)) => Ok(BuiltSource {
                    source,
                    name,
                    source_map,
                }),
                Err(failure) => Err(failure),
            },
        )
    }

    pub async fn write_binary(
        &self,
        computer: ComputerId,
        user: &str,
        folder: &str,
        name: &str,
        (source, allow_import, source_map): (&str, bool, SourceMap),
    ) -> Result<std::result::Result<(), BuildFailure>> {
        if self.networks.topology.computer(computer).await?.is_none() {
            return Ok(Err(BuildFailure::ComputerUnavailable));
        }

        let Some(folder_node) = self.files.lookup.folder(computer, folder, true).await? else {
            return Ok(Err(BuildFailure::MissingFolder(folder.to_owned())));
        };
        let account = self.computers.users.user(computer, user).await?;

        if !self
            .files
            .permissions
            .can_modify_contents(&folder_node, account.as_ref())
            .await?
        {
            return Ok(Err(BuildFailure::UnwritableFolder(folder.to_owned())));
        }

        let mut data = FileData::binary_from_source(source);

        data.allow_import = allow_import;
        data.source_map = source_map;
        self.generator
            .add_full_node(
                &folder_node,
                &NodeDraft::file(name, user, PermissionSet::default_for(false), data),
            )
            .await?;

        Ok(Ok(()))
    }

    pub async fn scp(
        &self,
        from: (ComputerId, &str),
        to: Option<(ComputerId, &str)>,
        source: &str,
        destination: &str,
        upload: bool,
    ) -> Result<std::result::Result<bool, greyhack_world_core::FileFailure>> {
        let default_user;
        let to = match to {
            Some(to) => to,
            None => {
                let player = self.world.player();

                default_user = self
                    .computers
                    .users
                    .users(player)
                    .await?
                    .get(1)
                    .map(|user| user.name.clone())
                    .unwrap_or_else(|| greyhack_behaviour::ROOT_USER.to_owned());

                (player, default_user.as_str())
            }
        };
        let (from, to) = if upload { (to, from) } else { (from, to) };

        Ok(self
            .files
            .changes
            .copy_between(from, source, to, destination)
            .await?
            .map(|report| report.skipped))
    }
}

impl BuildService {
    pub async fn source_code(
        &self,
        computer: ComputerId,
        user: &str,
        source: &str,
        folder: &str,
    ) -> Result<std::result::Result<(String, String), BuildFailure>> {
        let account = self.computers.users.user(computer, user).await?;
        let Some(source_node) = self.files.lookup.file(computer, source, true).await? else {
            return Ok(Err(BuildFailure::MissingSource(source.to_owned())));
        };
        let Some(folder_node) = self.files.lookup.folder(computer, folder, true).await? else {
            return Ok(Err(BuildFailure::MissingFolder(folder.to_owned())));
        };

        if !self
            .files
            .permissions
            .has_node_permission(&source_node, Permission::Read, account.as_ref())
            .await?
        {
            return Ok(Err(BuildFailure::UnreadableSource(source.to_owned())));
        }

        let Some(data) = source_node.data() else {
            return Ok(Err(BuildFailure::MissingSource(source.to_owned())));
        };

        if data.is_binary() {
            return Ok(Err(BuildFailure::BinarySource(
                source_node.name().to_owned(),
            )));
        }

        if !self
            .files
            .permissions
            .can_modify_contents(&folder_node, account.as_ref())
            .await?
        {
            return Ok(Err(BuildFailure::UnwritableFolder(folder.to_owned())));
        }

        let Some(name) = path::binary_name(source_node.name()) else {
            return Ok(Err(BuildFailure::InvalidExtension(
                source_node.name().to_owned(),
            )));
        };

        if data.content.is_empty() {
            return Ok(Err(BuildFailure::EmptySource));
        }

        Ok(Ok((data.content.clone(), name.to_owned())))
    }

    pub async fn expand_imports(
        &self,
        computer: Option<ComputerId>,
        user: &str,
        code: String,
    ) -> Result<std::result::Result<(String, SourceMap), BuildFailure>> {
        let Ok(paths) = import_paths(&code) else {
            return Ok(Err(BuildFailure::UnclosedImport));
        };
        let mut imports: Vec<Import> = Vec::with_capacity(paths.len());

        for import_path in paths {
            if !import_path.starts_with('/') {
                return Ok(Err(BuildFailure::Import(ImportFailure::RelativePath(
                    import_path,
                ))));
            }

            let content = match self.read_import(computer, user, &import_path).await? {
                Ok(Some(content)) => content,
                Ok(None) => {
                    return Ok(Err(BuildFailure::Import(ImportFailure::MissingFile(
                        import_path,
                    ))));
                }
                Err(failure) => return Ok(Err(BuildFailure::Import(failure))),
            };

            if imports.iter().any(|import| import.path == import_path) {
                return Ok(Err(BuildFailure::Import(ImportFailure::DuplicateImport(
                    import_path,
                ))));
            }

            let lines = u32::try_from(content.matches('\n').count() + 1).unwrap_or(u32::MAX);

            imports.push(Import {
                source_map: SourceMap::entire(path::file_name(&import_path), lines),
                path: import_path,
                code: content,
            });
        }

        if imports.is_empty() {
            return Ok(Ok((code, SourceMap::default())));
        }

        let lines = u32::try_from(code.matches('\n').count() + 1).unwrap_or(u32::MAX);

        Ok(put_imports(
            code,
            SourceMap::entire("", lines),
            &imports,
            Some(MAX_IMPORTED_CODE),
        )
        .map_err(|_| BuildFailure::Import(ImportFailure::TooLarge)))
    }

    pub async fn read_import(
        &self,
        computer: Option<ComputerId>,
        user: &str,
        file_path: &str,
    ) -> Result<std::result::Result<Option<String>, ImportFailure>> {
        let Some(computer) = computer else {
            return Ok(Ok(None));
        };
        let Some(node) = self.files.lookup.file(computer, file_path, true).await? else {
            return Ok(Ok(None));
        };
        let Some(data) = node.data() else {
            return Ok(Ok(None));
        };

        if data.is_binary() {
            if data.kind != FileKind::Program {
                return Ok(Err(ImportFailure::BinaryFile(file_path.to_owned())));
            }

            if !data.allow_import {
                return Ok(Err(ImportFailure::NoImportPermission(file_path.to_owned())));
            }
        }

        if !self
            .files
            .permissions
            .user_has_permission(computer, user, &node, Permission::Read)
            .await?
        {
            return Ok(Err(ImportFailure::Unreadable(file_path.to_owned())));
        }

        Ok(Ok(Some(data.content.clone())))
    }
}

/// A world service with only its declared dependencies.
pub struct BuildService {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
    pub(crate) generator: greyhack_world_modules::Dep<greyhack_world_libraries::Generator>,
}

impl BuildService {
    pub(crate) fn new(
        world: &greyhack_world_core::WorldRef,
        computers: greyhack_world_modules::Dep<greyhack_world_core::ComputerService>,
        files: greyhack_world_modules::Dep<greyhack_world_core::FileSystemService>,
        networks: greyhack_world_modules::Dep<greyhack_world_core::NetworkService>,
        generator: greyhack_world_modules::Dep<greyhack_world_libraries::Generator>,
    ) -> Self {
        Self {
            world: world.clone(),
            computers,
            files,
            networks,
            generator,
        }
    }
}
