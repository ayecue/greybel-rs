//! Computer file-system data: database [`Node`] rows and the in-memory [`FileSystemDraft`] used
//! during generation.

use greyhack_behaviour::path::ROOT;
use greyhack_behaviour::{
    FileData, MAX_SYMLINKS, METADATA_SIZE, Permission, PermissionSet, Principal, path,
};

use crate::model::{ComputerId, Library};
use crate::{FileFailure, User};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub(crate) u32);

/// A [`Node`] is either a folder with children or a file with data.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Folder,
    File(FileData),
}

/// Database row for a file or folder. The name is the final path segment. Children are ordered
/// folders first, then files, preserving insertion order within each group.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: NodeId,
    pub computer: ComputerId,
    pub parent: Option<NodeId>,
    /// Absolute path; `/` identifies the root folder.
    pub path: String,
    pub owner: String,
    pub group: String,
    pub permissions: PermissionSet,
    /// Symlink target, or an empty string for non-symlinks.
    pub symlink: String,
    pub protected: bool,
    /// World time of the last content change (`Fichero.timestampTicks`).
    pub timestamp: u64,
    pub kind: NodeKind,
}

impl Node {
    /// Protected root-level entries cannot be moved, renamed, or deleted.
    #[must_use]
    pub fn is_protected_in_root(&self) -> bool {
        self.protected && self.path != ROOT && path::parent(&self.path) == ROOT
    }

    /// Final path segment; `/` for the root folder.
    #[must_use]
    pub fn name(&self) -> &str {
        if self.path == ROOT {
            ROOT
        } else {
            path::file_name(&self.path)
        }
    }

    #[must_use]
    pub fn is_folder(&self) -> bool {
        self.permissions.is_folder()
    }

    #[must_use]
    pub fn is_symlink(&self) -> bool {
        !self.symlink.is_empty()
    }

    #[must_use]
    pub fn data(&self) -> Option<&FileData> {
        match &self.kind {
            NodeKind::File(data) => Some(data),
            NodeKind::Folder => None,
        }
    }

    pub fn data_mut(&mut self) -> Option<&mut FileData> {
        match &mut self.kind {
            NodeKind::File(data) => Some(data),
            NodeKind::Folder => None,
        }
    }

    #[must_use]
    pub fn can_read(&self, user: Option<&User>) -> bool {
        self.permissions.allows(
            Permission::Read,
            &self.owner,
            &self.group,
            user.map(|user| Principal::new(&user.name, &user.groups)),
        )
    }

    /// Returns whether `user` can write using `TienePermisoEscritura`; protected files always return false.
    #[must_use]
    pub fn can_write(&self, user: Option<&User>) -> bool {
        !self.protected
            && self.permissions.allows(
                Permission::Write,
                &self.owner,
                &self.group,
                user.map(|user| Principal::new(&user.name, &user.groups)),
            )
    }

    #[must_use]
    pub fn can_execute(&self, user: Option<&User>) -> bool {
        self.permissions.allows(
            Permission::Execute,
            &self.owner,
            &self.group,
            user.map(|user| Principal::new(&user.name, &user.groups)),
        )
    }

    /// Changes permissions as permitted by `UpdatePermisos` for `user`.
    ///
    /// # Errors
    ///
    /// Returns why `user` cannot change the permissions.
    pub fn try_set_permissions(
        &mut self,
        permissions: PermissionSet,
        user: Option<&User>,
    ) -> Result<(), FileFailure> {
        let Some(user) = user else {
            return Err(FileFailure::NoPermission);
        };

        if !user.is_root() && user.name != self.owner {
            return Err(FileFailure::NoPermission);
        }

        if self.protected {
            return Err(FileFailure::SystemFile);
        }

        self.permissions = permissions;

        Ok(())
    }

    /// Returns `GetSizeBytes` for a file or symlink, or the folder's own size excluding its
    /// contents (see `World::size`).
    #[must_use]
    pub fn own_size(&self, metadata: bool) -> u64 {
        if self.is_symlink() {
            return self.symlink.len() as u64;
        }

        match &self.kind {
            NodeKind::Folder if metadata => METADATA_SIZE,
            NodeKind::Folder => 0,
            NodeKind::File(data) if data.is_binary() || !metadata => data.size,
            NodeKind::File(data) => data.size + METADATA_SIZE,
        }
    }
}

/// Result of a completed filtered copy (`Fichero.FiltrarContenidoCopiable`).
#[derive(Debug, Clone, PartialEq)]
pub struct CopyReport {
    /// New root node, or the existing folder merged with the source.
    pub node: Node,
    /// Whether any source entry was skipped.
    pub skipped: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CopyPlan {
    pub(crate) new_entries: usize,
    pub(crate) file_size: u64,
    pub(crate) skipped: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FolderDraft {
    pub(crate) folders: Vec<NodeId>,
    pub(crate) files: Vec<NodeId>,
}

/// A [`NodeDraft`] is either a folder with children or a file with data.
#[derive(Debug, Clone, PartialEq)]
pub enum DraftKind {
    Folder(FolderDraft),
    File(FileData),
}

/// File or folder in a [`FileSystemDraft`].
#[derive(Debug, Clone, PartialEq)]
pub struct NodeDraft {
    pub name: String,
    pub(crate) parent: Option<NodeId>,
    pub owner: String,
    pub group: String,
    pub permissions: PermissionSet,
    /// Symlink target, or an empty string for non-symlinks.
    pub symlink: String,
    pub protected: bool,
    /// World time of the last content change. Zero requests an inherited initial age from `add`.
    pub timestamp: u64,
    pub kind: DraftKind,
    /// Library weakness data used by `metaxploit` and `overflow`.
    pub library: Option<Box<Library>>,
}

impl NodeDraft {
    /// Creates a folder with `owner` and its matching group.
    #[must_use]
    pub fn folder(name: &str, owner: &str, permissions: PermissionSet) -> Self {
        Self::new(
            name,
            owner,
            permissions,
            DraftKind::Folder(FolderDraft::default()),
        )
    }

    /// Creates a file with `owner` and its matching group.
    #[must_use]
    pub fn file(name: &str, owner: &str, permissions: PermissionSet, mut data: FileData) -> Self {
        if data.kind == greyhack_behaviour::FileKind::Binary
            && data.command.is_none()
            && data.command_name.is_empty()
        {
            // `Archivo(string, string, string, bool, bool)` names an otherwise unconfigured binary
            // after its file.
            data.command_name = name.to_owned();
        }

        Self::new(name, owner, permissions, DraftKind::File(data))
    }

    fn new(name: &str, owner: &str, permissions: PermissionSet, kind: DraftKind) -> Self {
        Self {
            name: name.to_owned(),
            parent: None,
            owner: owner.to_owned(),
            group: owner.to_owned(),
            permissions,
            symlink: String::new(),
            protected: false,
            timestamp: 0,
            kind,
            library: None,
        }
    }

    #[must_use]
    pub fn is_folder(&self) -> bool {
        self.permissions.is_folder()
    }

    #[must_use]
    pub fn is_symlink(&self) -> bool {
        !self.symlink.is_empty()
    }

    #[must_use]
    pub fn data(&self) -> Option<&FileData> {
        match &self.kind {
            DraftKind::File(data) => Some(data),
            DraftKind::Folder(_) => None,
        }
    }

    pub fn data_mut(&mut self) -> Option<&mut FileData> {
        match &mut self.kind {
            DraftKind::File(data) => Some(data),
            DraftKind::Folder(_) => None,
        }
    }

    /// Returns the file's `Library` weakness data, if present.
    #[must_use]
    pub fn library(&self) -> Option<&Library> {
        self.library.as_deref()
    }
}

/// Generated ordered file tree addressed by [`NodeId`]. IDs are draft indexes, not database row
/// IDs. The complete tree is inserted into the database.
#[derive(Debug, Clone, PartialEq)]
pub struct FileSystemDraft {
    nodes: Vec<Option<NodeDraft>>,
    absolute_dates: bool,
}

impl Default for FileSystemDraft {
    fn default() -> Self {
        Self::new()
    }
}

impl FileSystemDraft {
    const ROOT: NodeId = NodeId(0);

    /// Creates a file system containing only a root-owned `drwxr-xr-x` root folder
    /// (`FileSystem.CarpetaRaiz` in `XmlGlobal.BaseFileSystem`).
    #[must_use]
    pub fn new() -> Self {
        let mut root = NodeDraft::folder(
            ROOT,
            greyhack_behaviour::ROOT_USER,
            PermissionSet::parse("drwxr-xr-x").expect("a valid permission set"),
        );

        root.timestamp = greyhack_behaviour::parse_date(greyhack_behaviour::START_DATE)
            .saturating_sub(greyhack_behaviour::SECONDS_PER_DAY);

        Self {
            nodes: vec![Some(root)],
            absolute_dates: false,
        }
    }

    /// Marks the dates of the nodes as the dates the files have, which insertion does not shift by the stable
    /// per-computer offset of a template clone (`XmlGlobal.CloneTemplate`). A copy of a computer that already exists
    /// uses it, as its files are dated already.
    #[must_use]
    pub fn with_absolute_dates(mut self) -> Self {
        self.absolute_dates = true;

        self
    }

    /// Whether the dates of the nodes are final, see [`Self::with_absolute_dates`].
    #[must_use]
    pub fn has_absolute_dates(&self) -> bool {
        self.absolute_dates
    }

    #[must_use]
    pub fn root(&self) -> NodeId {
        Self::ROOT
    }

    /// # Panics
    ///
    /// Panics when `id` references a removed node.
    #[must_use]
    pub fn node(&self, id: NodeId) -> &NodeDraft {
        self.nodes[id.0 as usize].as_ref().expect("a live node")
    }

    /// # Panics
    ///
    /// Panics when `id` references a removed node.
    pub fn node_mut(&mut self, id: NodeId) -> &mut NodeDraft {
        self.nodes[id.0 as usize].as_mut().expect("a live node")
    }

    fn folder_lists(&self, id: NodeId) -> Option<&FolderDraft> {
        match &self.node(id).kind {
            DraftKind::Folder(folder) => Some(folder),
            DraftKind::File(_) => None,
        }
    }

    fn folder_lists_mut(&mut self, id: NodeId) -> Option<&mut FolderDraft> {
        match &mut self.node_mut(id).kind {
            DraftKind::Folder(folder) => Some(folder),
            DraftKind::File(_) => None,
        }
    }

    /// Returns subfolders in insertion order.
    #[must_use]
    pub fn folders(&self, id: NodeId) -> &[NodeId] {
        self.folder_lists(id).map_or(&[], |folder| &folder.folders)
    }

    /// Returns files in insertion order.
    #[must_use]
    pub fn files(&self, id: NodeId) -> &[NodeId] {
        self.folder_lists(id).map_or(&[], |folder| &folder.files)
    }

    /// Returns subfolders before files in `GetFicheros` order.
    #[must_use]
    pub fn children(&self, id: NodeId) -> Vec<NodeId> {
        [self.folders(id), self.files(id)].concat()
    }

    fn child_with_name(&self, folder: NodeId, name: &str) -> Option<NodeId> {
        self.children(folder)
            .into_iter()
            .find(|child| self.node(*child).name == name)
    }

    #[must_use]
    pub fn has_child(&self, folder: NodeId, name: &str) -> bool {
        self.child_with_name(folder, name).is_some()
    }

    #[must_use]
    pub fn path(&self, id: NodeId) -> String {
        let mut names = Vec::new();
        let mut current = Some(id);

        while let Some(node) = current.filter(|node| *node != Self::ROOT) {
            names.push(self.node(node).name.as_str());
            current = self.node(node).parent;
        }

        names.reverse();

        format!("/{}", names.join("/"))
    }

    /// Resolves a folder from the root, optionally following its final symlink.
    #[must_use]
    pub fn folder(&self, path: &str, follow: bool) -> Option<NodeId> {
        self.folder_from(path, Self::ROOT, follow, 0)
    }

    fn folder_from(&self, path: &str, start: NodeId, follow: bool, links: u32) -> Option<NodeId> {
        if path.is_empty() {
            return None;
        }

        if path == ROOT {
            return Some(Self::ROOT);
        }

        let segments = path::split(path)?;
        let segment_folder = self
            .folders(start)
            .iter()
            .copied()
            .find(|folder| self.node(*folder).name == segments[0])?;

        if segments.len() > 1 {
            let remainder = format!("/{}", segments[1..].join("/"));

            return self.folder_from(&remainder, segment_folder, follow, links);
        }

        let node = self.node(segment_folder);

        if !follow || !node.is_symlink() {
            return Some(segment_folder);
        }

        if node.symlink == self.path(segment_folder) || links > MAX_SYMLINKS {
            return None;
        }

        if self.folder(&node.symlink, false).is_none() {
            return Some(segment_folder);
        }

        self.folder_from(&node.symlink, Self::ROOT, true, links + 1)
    }

    /// Returns the file at `path` using `GetArchivo`, following symlinks in parent segments.
    #[must_use]
    pub fn file(&self, path: &str, follow: bool) -> Option<NodeId> {
        self.file_at(path, follow, 0)
    }

    fn file_at(&self, path: &str, follow: bool, links: u32) -> Option<NodeId> {
        if path.is_empty() {
            return None;
        }

        let folder = self.folder(path::parent(path), true)?;
        let name = path::file_name(path);
        let file_id = self
            .files(folder)
            .iter()
            .copied()
            .find(|file| self.node(*file).name == name)?;
        let node = self.node(file_id);

        if !follow || !node.is_symlink() {
            return Some(file_id);
        }

        if node.symlink == self.path(file_id) || links > MAX_SYMLINKS {
            return None;
        }

        if self.file(&node.symlink, false).is_none() {
            return Some(file_id);
        }

        self.file_at(&node.symlink, true, links + 1)
    }

    #[must_use]
    pub fn library(&self, path: &str, follow: bool) -> Option<&Library> {
        self.node(self.file(path, follow)?).library()
    }

    /// Adds `node` to `folder`, replacing an entry with the same name.
    ///
    /// # Panics
    ///
    /// Panics if `folder` refers to a file.
    pub fn add(&mut self, folder: NodeId, mut node: NodeDraft) -> NodeId {
        if let Some(same_name_child) = self.child_with_name(folder, &node.name) {
            self.remove(same_name_child);
        }

        node.parent = Some(folder);

        if node.timestamp == 0 && self.node(folder).timestamp != 0 {
            node.timestamp =
                greyhack_behaviour::template_timestamp(self.node(folder).timestamp, &node.name);
        }

        let is_folder = node.is_folder();
        let id = NodeId(u32::try_from(self.nodes.len()).expect("fewer than 2^32 nodes"));

        self.nodes.push(Some(node));

        let lists = self.folder_lists_mut(folder).expect("adding into a folder");

        if is_folder {
            lists.folders.push(id);
        } else {
            lists.files.push(id);
        }

        id
    }

    /// Gives every node that belongs to `old` to `new`, as renaming an account does.
    pub(crate) fn rename_owner(&mut self, old: &str, new: &str) {
        for node in self.nodes.iter_mut().flatten() {
            if node.owner == old {
                new.clone_into(&mut node.owner);
            }

            if node.group == old {
                new.clone_into(&mut node.group);
            }
        }
    }

    /// Removes `id` and its descendants. Returns false for the root or a removed node.
    pub fn remove(&mut self, id: NodeId) -> bool {
        let Some(parent) = self.nodes[id.0 as usize]
            .as_ref()
            .and_then(|node| node.parent)
        else {
            return false;
        };

        if let Some(lists) = self.folder_lists_mut(parent) {
            lists.folders.retain(|child| *child != id);
            lists.files.retain(|child| *child != id);
        }

        let mut node_stack = vec![id];

        while let Some(node) = node_stack.pop() {
            node_stack.extend(self.children(node));

            self.nodes[node.0 as usize] = None;
        }

        true
    }
}
