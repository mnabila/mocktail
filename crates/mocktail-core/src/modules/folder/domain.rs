//! Folder aggregate: Postman-style grouping of mocks inside a collection.
//! Folders are a separate aggregate; mocks reference a folder by id.

use crate::shared::ids::{CollectionId, FolderId, WorkspaceId};
use crate::shared::ports::UnixMillis;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    pub id: FolderId,
    pub workspace_id: WorkspaceId,
    pub collection_id: CollectionId,
    pub name: String,
    pub parent_id: Option<FolderId>,
    pub created_at: UnixMillis,
}
