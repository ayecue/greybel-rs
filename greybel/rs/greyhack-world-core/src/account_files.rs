//! Account-file operations supplied while core constructs its user and file services.

use crate::{CallFuture, ComputerId, Node, NodeDraft};

/// Account-file operations used by `UserService` while file services depend on `UserService`.
///
/// `CoreModule::new` registers the `FileSystemService` implementation in the caller's registry, breaking the
/// construction cycle without a back-pointer to the module or its repository aggregate.
pub(crate) trait AccountFileService {
    /// Returns the computer's root folder.
    fn root(&self, computer: ComputerId) -> CallFuture<'_, Option<Node>>;

    /// Returns a file by path.
    fn file<'a>(
        &'a self,
        computer: ComputerId,
        path: &'a str,
        follow: bool,
    ) -> CallFuture<'a, Option<Node>>;

    /// Returns a folder by path.
    fn folder<'a>(
        &'a self,
        computer: ComputerId,
        path: &'a str,
        follow: bool,
    ) -> CallFuture<'a, Option<Node>>;

    /// Returns a folder's direct subfolders.
    fn folders<'a>(&'a self, folder: &'a Node) -> CallFuture<'a, Vec<Node>>;

    /// Adds an account file or folder.
    fn add_node<'a>(&'a self, folder: &'a Node, node: &'a NodeDraft) -> CallFuture<'a, Node>;

    /// Removes an account file or folder.
    fn remove_node<'a>(&'a self, node: &'a Node) -> CallFuture<'a, bool>;

    /// Saves an account file or folder.
    fn save_node<'a>(&'a self, node: &'a Node) -> CallFuture<'a, ()>;
}
