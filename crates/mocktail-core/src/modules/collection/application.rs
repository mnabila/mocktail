//! Collection use cases + repository port.

use super::domain::{Collection, CorsPolicy, Source, normalize_base_path};
use crate::shared::error::{AppError, AppResult};
use crate::shared::ids::{CollectionId, Slug, WorkspaceId};
use crate::shared::ports::UnixMillis;
use async_trait::async_trait;

#[async_trait]
pub trait CollectionRepository: Send + Sync {
    async fn find_by_slug(&self, ws: &WorkspaceId, slug: &Slug) -> AppResult<Option<Collection>>;
    async fn find_by_id(&self, id: &CollectionId) -> AppResult<Option<Collection>>;
    async fn list(&self, ws: &WorkspaceId) -> AppResult<Vec<Collection>>;
    async fn insert(&self, collection: &Collection) -> AppResult<()>;
    async fn update(&self, collection: &Collection) -> AppResult<()>;
    async fn delete(&self, id: &CollectionId) -> AppResult<()>;
}

pub struct NewCollection {
    pub name: String,
    pub slug: Slug,
    pub description: String,
    pub source: Source,
    pub base_path: String,
    pub cors_policy: Option<CorsPolicy>,
}

/// Create a collection; slug is unique per workspace.
pub async fn create_collection(
    repo: &dyn CollectionRepository,
    id: CollectionId,
    ws: WorkspaceId,
    input: NewCollection,
    now: UnixMillis,
) -> AppResult<Collection> {
    if input.name.trim().is_empty() {
        return Err(AppError::BadRequest("collection name is required".into()));
    }
    if repo.find_by_slug(&ws, &input.slug).await?.is_some() {
        return Err(AppError::Conflict(format!(
            "collection slug {:?} already exists in this workspace",
            input.slug
        )));
    }
    let collection = Collection {
        id,
        workspace_id: ws,
        name: input.name,
        slug: input.slug,
        description: input.description,
        source: input.source,
        base_path: normalize_base_path(&input.base_path),
        cors_policy: input.cors_policy.unwrap_or_default(),
        version: 1,
        created_at: now,
    };
    repo.insert(&collection).await?;
    Ok(collection)
}

/// Load a collection or 404.
pub async fn get_collection(
    repo: &dyn CollectionRepository,
    id: &CollectionId,
) -> AppResult<Collection> {
    repo.find_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("collection {id}")))
}

/// Apply an edit and bump the version so cached mock sets are invalidated.
pub async fn update_collection(
    repo: &dyn CollectionRepository,
    mut collection: Collection,
) -> AppResult<Collection> {
    if collection.name.trim().is_empty() {
        return Err(AppError::BadRequest("collection name is required".into()));
    }
    if let Some(existing) = repo
        .find_by_slug(&collection.workspace_id, &collection.slug)
        .await?
        && existing.id != collection.id
    {
        return Err(AppError::Conflict(format!(
            "collection slug {:?} already exists in this workspace",
            collection.slug
        )));
    }
    collection.name = collection.name.trim().to_string();
    collection.version += 1;
    repo.update(&collection).await?;
    Ok(collection)
}

pub async fn delete_collection(
    repo: &dyn CollectionRepository,
    id: &CollectionId,
) -> AppResult<()> {
    repo.delete(id).await
}
