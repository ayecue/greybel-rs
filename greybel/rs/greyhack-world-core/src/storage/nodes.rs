//! File and folder rows keyed by computer and full path. Folder children are ordered folders first,
//! then files, preserving insertion order within each group.

use greybel_source_map::SourceMap;
use greyhack_behaviour::path::ROOT;
use greyhack_behaviour::{Command, FileData, FileKind, PermissionSet, path};
use turso::params;

use super::Database;
use super::sql::{FromRow, RowView, execute, fetch_all, fetch_opt, fetch_value, insert};
use crate::model::DraftKind;
use crate::{ComputerId, FileSystemDraft, LibraryId, Node, NodeDraft, NodeId, NodeKind, Result};

/// Folder value for the `kind` column; files store their [`FileKind`] discriminant.
const FOLDER: i64 = 0;

/// Node columns and associated file command.
macro_rules! node_columns {
    () => {
        "nodes.id, nodes.computer_id, nodes.parent_id, nodes.path, nodes.seq, nodes.owner, nodes.grp, \
         nodes.permissions, nodes.symlink, nodes.protected, nodes.timestamp, nodes.kind, nodes.content, \
         node_commands.command, nodes.size, nodes.allow_import, nodes.encrypt_password, nodes.source_map, \
         nodes.default_content, nodes.command_string FROM nodes \
         LEFT JOIN node_commands ON node_commands.node_id = nodes.id"
    };
}

impl FromRow for Node {
    fn from_row(row: &RowView<'_>) -> turso::Result<Self> {
        let code: i64 = row.get("kind")?;
        let is_folder = code == FOLDER;
        let kind = if is_folder {
            NodeKind::Folder
        } else {
            NodeKind::File(FileData {
                kind: FileKind::from_code(code).unwrap_or(FileKind::Binary),
                content: row.get::<Option<String>>("content")?.unwrap_or_default(),
                command: row.get_code_opt("command")?,
                command_name: row
                    .get::<Option<String>>("command_string")?
                    .unwrap_or_default(),
                size: row.get::<Option<u64>>("size")?.unwrap_or(0),
                allow_import: row.get::<Option<bool>>("allow_import")?.unwrap_or_default(),
                encrypt_password: row
                    .get::<Option<String>>("encrypt_password")?
                    .unwrap_or_default(),
                source_map: row
                    .get::<Option<String>>("source_map")?
                    .map(|source_map| read_source_map(&source_map))
                    .unwrap_or_default(),
                default_content: row
                    .get::<Option<bool>>("default_content")?
                    .unwrap_or_default(),
            })
        };

        Ok(Self {
            id: NodeId(row.get("id")?),
            computer: ComputerId(row.get("computer_id")?),
            parent: row.get::<Option<u32>>("parent_id")?.map(NodeId),
            path: row.get("path")?,
            owner: row.get("owner")?,
            group: row.get("grp")?,
            permissions: PermissionSet::parse(&row.get::<String>("permissions")?)
                .unwrap_or_else(|| PermissionSet::default_for(is_folder)),
            symlink: row.get("symlink")?,
            protected: row.get("protected")?,
            timestamp: row.get::<Option<u64>>("timestamp")?.unwrap_or_default(),
            kind,
        })
    }
}

/// Returns the `source_map` column for `data`, or null without a source map.
fn source_map_column(data: Option<&FileData>) -> Option<String> {
    let source_map = &data?.source_map;

    (!source_map.is_empty())
        .then(|| serde_json::to_string(source_map).expect("source maps serialize"))
}

/// Parses a `source_map` column. Invalid values produce an empty map.
fn read_source_map(column: &str) -> SourceMap {
    serde_json::from_str(column).unwrap_or_default()
}

/// Returns the `kind` column for node `data`; folders have no kind value.
fn kind_code(data: Option<&FileData>) -> i64 {
    data.map_or(FOLDER, |data| data.kind.code())
}

/// Returns path bounds below `path`: from `path/` inclusive to `path0` exclusive (`0` sorts
/// immediately after `/`).
fn below(path: &str) -> (String, String) {
    if path == ROOT {
        ("/".to_owned(), "0".to_owned())
    } else {
        (format!("{path}/"), format!("{path}0"))
    }
}

/// New node data excluding its location.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeSpec<'a> {
    pub owner: &'a str,
    pub group: &'a str,
    pub permissions: PermissionSet,
    pub symlink: &'a str,
    pub protected: bool,
    pub timestamp: u64,
    pub data: Option<&'a FileData>,
}

impl<'a> NodeSpec<'a> {
    /// Fields from a stored `node`.
    pub fn of(node: &'a Node) -> Self {
        Self {
            owner: &node.owner,
            group: &node.group,
            permissions: node.permissions,
            symlink: &node.symlink,
            protected: node.protected,
            timestamp: node.timestamp,
            data: node.data(),
        }
    }

    /// Fields from a draft node.
    pub fn of_draft<L>(node: &'a NodeDraft<L>) -> Self {
        Self {
            owner: &node.owner,
            group: &node.group,
            permissions: node.permissions,
            symlink: &node.symlink,
            protected: node.protected,
            timestamp: node.timestamp,
            data: node.data(),
        }
    }
}

/// Queries of the tables owned by this area.
pub(crate) struct NodeRepository {
    db: Database,
}

impl NodeRepository {
    pub(crate) fn new(db: Database) -> Self {
        Self { db }
    }

    /// Returns computer node `id`, or `None` if removed.
    pub async fn node(&self, computer: ComputerId, id: NodeId) -> Result<Option<Node>> {
        Ok(fetch_opt(
            &self.db.connection(),
            concat!(
                "SELECT ",
                node_columns!(),
                " WHERE nodes.computer_id = ?1 AND nodes.id = ?2"
            ),
            params![computer.0, id.0],
        )
        .await?)
    }

    /// Returns the computer node at exact `path` without following symlinks.
    pub async fn node_at(&self, computer: ComputerId, path: &str) -> Result<Option<Node>> {
        Ok(fetch_opt(
            &self.db.connection(),
            concat!(
                "SELECT ",
                node_columns!(),
                " WHERE nodes.computer_id = ?1 AND nodes.path = ?2"
            ),
            params![computer.0, path],
        )
        .await?)
    }

    /// Returns the computer root folder.
    pub async fn root(&self, computer: ComputerId) -> Result<Option<Node>> {
        self.node_at(computer, ROOT).await
    }

    /// Returns folder children, folders before files, each group in insertion order.
    pub async fn children(&self, computer: ComputerId, folder: NodeId) -> Result<Vec<Node>> {
        Ok(fetch_all(
            &self.db.connection(),
            concat!(
                "SELECT ",
                node_columns!(),
                " WHERE nodes.computer_id = ?1 AND nodes.parent_id = ?2 ORDER BY nodes.kind = 0 DESC, nodes.seq"
            ),
            params![computer.0, folder.0],
        )
        .await?)
    }

    /// Returns descendants of computer `path`, parents before children and siblings in
    /// [`Self::children`] order.
    pub async fn tree_below(&self, computer: ComputerId, path: &str) -> Result<Vec<Node>> {
        let (from, to) = below(path);
        let mut nodes: Vec<Node> = fetch_all(
            &self.db.connection(),
            concat!(
                "SELECT ",
                node_columns!(),
                " WHERE nodes.computer_id = ?1 AND nodes.path >= ?2 AND nodes.path < ?3 ORDER BY nodes.seq"
            ),
            params![computer.0, from.as_str(), to.as_str()],
        )
        .await?;
        let mut result = Vec::with_capacity(nodes.len());
        let mut stack = vec![path.to_owned()];

        // Depth-first traversal from `path`: each folder's subfolders, then files, in insertion
        // order.
        while let Some(parent) = stack.pop() {
            let (mut folders, mut files): (Vec<Node>, Vec<Node>) = nodes
                .extract_if(.., |node| {
                    greyhack_behaviour::path::parent(&node.path) == parent && node.path != parent
                })
                .partition(Node::is_folder);

            for folder in folders.iter().rev() {
                stack.push(folder.path.clone());
            }

            result.append(&mut folders);
            result.append(&mut files);
        }

        Ok(result)
    }

    /// Counts nodes below computer `path` (`GetTotalFicheros`).
    pub async fn count_below(&self, computer: ComputerId, path: &str) -> Result<usize> {
        let (from, to) = below(path);
        let count: i64 = fetch_value(
            &self.db.connection(),
            "SELECT count(*) FROM nodes WHERE computer_id = ?1 AND path >= ?2 AND path < ?3",
            params![computer.0, from.as_str(), to.as_str()],
        )
        .await?
        .unwrap_or(0);

        Ok(usize::try_from(count).unwrap_or(0))
    }

    /// Inserts node `name` under computer `parent` at `parent_path`, after existing children, and
    /// returns its ID. The name must be unique in that folder.
    pub async fn insert_node(
        &self,
        computer: ComputerId,
        parent: Option<(NodeId, &str)>,
        name: &str,
        fields: &NodeSpec<'_>,
    ) -> Result<NodeId> {
        let path = parent.map_or_else(|| ROOT.to_owned(), |(_, folder)| path::join(folder, name));
        let data = fields.data;
        let id = insert(
            &self.db.connection(),
            "INSERT INTO nodes (computer_id, parent_id, path, seq, kind, owner, grp, permissions, symlink, protected, \
             timestamp, content, size, allow_import, encrypt_password, source_map, default_content, command_string) \
             VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(seq), 0) + 1 FROM nodes WHERE computer_id = ?1), ?4, ?5, ?6, \
             ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                computer.0,
                parent.map(|(id, _)| id.0),
                path.as_str(),
                kind_code(data),
                fields.owner,
                fields.group,
                fields.permissions.as_str(),
                fields.symlink,
                fields.protected,
                i64::try_from(fields.timestamp).unwrap_or(i64::MAX),
                data.map(|data| data.content.as_str()),
                data.map(|data| i64::try_from(data.size).unwrap_or(i64::MAX)),
                data.map(|data| data.allow_import),
                data.map(|data| data.encrypt_password.as_str()),
                source_map_column(data),
                data.is_some_and(|data| data.default_content),
                data.map_or("", |data| data.command_name.as_str()),
            ],
        )
        .await?;
        let id = NodeId(u32::try_from(id).expect("node ids fit u32"));

        self.set_command(id, data.and_then(|data| data.command))
            .await?;

        Ok(id)
    }

    /// Updates owner, group, permissions, symlink, protection, and file data. Location changes use
    /// [`Self::move_node`].
    pub async fn save_node(&self, node: &Node) -> Result<()> {
        let data = node.data();

        execute(
            &self.db.connection(),
            "UPDATE nodes SET kind = ?3, owner = ?4, grp = ?5, permissions = ?6, symlink = ?7, protected = ?8, \
             timestamp = ?9, content = ?10, size = ?11, allow_import = ?12, encrypt_password = ?13, source_map = ?14, \
             default_content = ?15, command_string = ?16 WHERE computer_id = ?1 AND id = ?2",
            params![
                node.computer.0,
                node.id.0,
                kind_code(data),
                node.owner.as_str(),
                node.group.as_str(),
                node.permissions.as_str(),
                node.symlink.as_str(),
                node.protected,
                i64::try_from(node.timestamp).unwrap_or(i64::MAX),
                data.map(|data| data.content.as_str()),
                data.map(|data| i64::try_from(data.size).unwrap_or(i64::MAX)),
                data.map(|data| data.allow_import),
                data.map(|data| data.encrypt_password.as_str()),
                source_map_column(data),
                data.is_some_and(|data| data.default_content),
                data.map_or("", |data| data.command_name.as_str()),
            ],
        )
        .await?;

        self.set_command(node.id, data.and_then(|data| data.command))
            .await
    }

    /// Sets or clears the command started by file `node`.
    async fn set_command(&self, node: NodeId, command: Option<Command>) -> Result<()> {
        match command {
            Some(command) => {
                execute(
                    &self.db.connection(),
                    "INSERT INTO node_commands (node_id, command) VALUES (?1, ?2) \
                 ON CONFLICT (node_id) DO UPDATE SET command = ?2",
                    params![node.0, command.code()],
                )
                .await?
            }
            None => {
                execute(
                    &self.db.connection(),
                    "DELETE FROM node_commands WHERE node_id = ?1",
                    params![node.0],
                )
                .await?
            }
        };

        Ok(())
    }

    /// Deletes computer node `id`, its descendants, libraries, and debug data.
    pub async fn remove_node(&self, computer: ComputerId, id: NodeId) -> Result<()> {
        execute(
            &self.db.connection(),
            "DELETE FROM nodes WHERE computer_id = ?1 AND id = ?2",
            params![computer.0, id.0],
        )
        .await?;

        Ok(())
    }

    /// Inserts `draft`, resolving each library through `library_id` at the same point in the write
    /// order. The libraries domain uses this to persist zones and weaknesses with the identity.
    pub async fn insert_filesystem_with<L>(
        &self,
        computer: ComputerId,
        draft: &FileSystemDraft<L>,
        library_id: impl AsyncFn(&L) -> Result<LibraryId>,
    ) -> Result<()> {
        // `XmlGlobal.CloneTemplate`: shifts each computer template by an FNV-1a hash of its stable
        // identifier.
        let offset_minutes = if draft.has_absolute_dates() {
            0
        } else {
            1 + u64::from(greyhack_behaviour::timestamp_hash(&computer.0.to_string()) % 1_051_200)
        };
        let offset_seconds = offset_minutes * 60;
        let root = draft.root();
        let root_node = draft.node(root);
        let mut root_fields = NodeSpec::of_draft(root_node);

        root_fields.timestamp = root_fields.timestamp.saturating_sub(offset_seconds);

        let root_id = self.insert_node(computer, None, ROOT, &root_fields).await?;
        let mut stack = vec![(root, root_id, ROOT.to_owned())];

        while let Some((draft_folder, folder, folder_path)) = stack.pop() {
            for child in draft.children(draft_folder) {
                let node = draft.node(child);
                let mut fields = NodeSpec::of_draft(node);

                fields.timestamp = fields.timestamp.saturating_sub(offset_seconds);

                let id = self
                    .insert_node(computer, Some((folder, &folder_path)), &node.name, &fields)
                    .await?;

                if let Some(library) = node.library() {
                    let library = library_id(library).await?;

                    self.set_node_library(id, Some(library)).await?;
                }

                if matches!(node.kind, DraftKind::Folder(_)) {
                    stack.push((child, id, path::join(&folder_path, &node.name)));
                }
            }
        }

        Ok(())
    }

    /// Returns the library associated with file `node`.
    pub async fn node_library(&self, node: NodeId) -> Result<Option<LibraryId>> {
        Ok(fetch_value::<u32>(
            &self.db.connection(),
            "SELECT library_id FROM node_libraries WHERE node_id = ?1",
            params![node.0],
        )
        .await?
        .map(LibraryId))
    }

    /// Associates `library` with file `node`, or removes the association.
    pub async fn set_node_library(&self, node: NodeId, library: Option<LibraryId>) -> Result<()> {
        match library {
            Some(library) => {
                execute(
                    &self.db.connection(),
                    "INSERT INTO node_libraries (node_id, library_id) VALUES (?1, ?2) \
                 ON CONFLICT (node_id) DO UPDATE SET library_id = ?2",
                    params![node.0, library.0],
                )
                .await?
            }
            None => {
                execute(
                    &self.db.connection(),
                    "DELETE FROM node_libraries WHERE node_id = ?1",
                    params![node.0],
                )
                .await?
            }
        };

        Ok(())
    }
}
