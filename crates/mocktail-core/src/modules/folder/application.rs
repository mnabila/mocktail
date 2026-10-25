//! Folder use cases + repository port.

use super::domain::Folder;
use crate::modules::mock::application::MockRepository;
use crate::shared::error::{AppError, AppResult};
use crate::shared::ids::{CollectionId, FolderId, WorkspaceId};
use crate::shared::ports::UnixMillis;
use async_trait::async_trait;
use std::collections::HashSet;

#[async_trait]
pub trait FolderRepository: Send + Sync {
    async fn list(&self, ws: &WorkspaceId, coll: &CollectionId) -> AppResult<Vec<Folder>>;
    async fn find_by_id(&self, id: &FolderId) -> AppResult<Option<Folder>>;
    async fn insert(&self, folder: &Folder) -> AppResult<()>;
    async fn delete(&self, id: &FolderId) -> AppResult<()>;
}

pub struct NewFolder {
    pub name: String,
    pub parent_id: Option<FolderId>,
}

pub async fn create_folder(
    repo: &dyn FolderRepository,
    id: FolderId,
    ws: WorkspaceId,
    coll: CollectionId,
    input: NewFolder,
    now: UnixMillis,
) -> AppResult<Folder> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("folder name is required".into()));
    }
    if let Some(parent_id) = &input.parent_id {
        let parent = repo
            .find_by_id(parent_id)
            .await?
            .ok_or_else(|| AppError::BadRequest("parent folder not found".into()))?;
        if parent.collection_id != coll {
            return Err(AppError::BadRequest(
                "parent folder belongs to another collection".into(),
            ));
        }
    }
    let folder = Folder {
        id,
        workspace_id: ws,
        collection_id: coll,
        name,
        parent_id: input.parent_id,
        created_at: now,
    };
    repo.insert(&folder).await?;
    Ok(folder)
}

pub async fn list_folders(
    repo: &dyn FolderRepository,
    ws: &WorkspaceId,
    coll: &CollectionId,
) -> AppResult<Vec<Folder>> {
    repo.list(ws, coll).await
}

pub async fn get_folder(
    repo: &dyn FolderRepository,
    coll: &CollectionId,
    id: &FolderId,
) -> AppResult<Folder> {
    let folder = repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("folder {id}")))?;
    if &folder.collection_id != coll {
        return Err(AppError::NotFound(format!("folder {id}")));
    }
    Ok(folder)
}

/// Delete a folder and its subtree. Mocks in any removed folder fall back to
/// the collection root rather than becoming invisible.
pub async fn delete_folder(
    folders: &dyn FolderRepository,
    mocks: &dyn MockRepository,
    folder: &Folder,
) -> AppResult<()> {
    let all = folders
        .list(&folder.workspace_id, &folder.collection_id)
        .await?;

    // Breadth-first subtree collection. A visited set also guards against
    // malformed/legacy parent cycles.
    let mut doomed: Vec<FolderId> = vec![folder.id.clone()];
    let mut seen: HashSet<String> = HashSet::from([folder.id.as_str().to_string()]);
    let mut i = 0;
    while i < doomed.len() {
        let parent = doomed[i].clone();
        i += 1;
        for candidate in &all {
            if candidate.parent_id.as_ref() == Some(&parent)
                && seen.insert(candidate.id.as_str().to_string())
            {
                doomed.push(candidate.id.clone());
            }
        }
    }
    let ids: HashSet<String> = doomed.iter().map(|id| id.as_str().to_string()).collect();

    for mock in mocks
        .list_active(&folder.workspace_id, &folder.collection_id)
        .await?
    {
        if mock
            .folder_id
            .as_ref()
            .is_some_and(|id| ids.contains(id.as_str()))
        {
            let mut updated = mock;
            updated.folder_id = None;
            mocks.upsert(&updated).await?;
        }
    }

    // Children before parents (safe if an adapter ever enforces a foreign key).
    for id in doomed.into_iter().rev() {
        folders.delete(&id).await?;
    }
    Ok(())
}
