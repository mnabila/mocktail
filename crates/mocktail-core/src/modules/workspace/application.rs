//! Workspace use cases + repository port (DDD: one repository per aggregate).

use super::domain::Workspace;
use crate::shared::error::{AppError, AppResult};
use crate::shared::ids::{Slug, WorkspaceId};
use async_trait::async_trait;

#[async_trait]
pub trait WorkspaceRepository: Send + Sync {
    async fn find_by_slug(&self, slug: &Slug) -> AppResult<Option<Workspace>>;
    async fn find_by_id(&self, id: &WorkspaceId) -> AppResult<Option<Workspace>>;
    async fn insert(&self, workspace: &Workspace) -> AppResult<()>;
    async fn update(&self, workspace: &Workspace) -> AppResult<()>;
    async fn delete(&self, id: &WorkspaceId) -> AppResult<()>;
    async fn list(&self) -> AppResult<Vec<Workspace>>;
}

/// Create a workspace, enforcing slug uniqueness per deployment.
pub async fn create_workspace(
    repo: &dyn WorkspaceRepository,
    id: WorkspaceId,
    name: String,
    slug: Slug,
    now: crate::shared::ports::UnixMillis,
) -> AppResult<Workspace> {
    if name.trim().is_empty() {
        return Err(AppError::BadRequest("workspace name is required".into()));
    }
    if repo.find_by_slug(&slug).await?.is_some() {
        return Err(AppError::Conflict(format!(
            "workspace slug {slug:?} already exists"
        )));
    }
    let workspace = Workspace {
        id,
        name,
        slug,
        created_at: now,
    };
    repo.insert(&workspace).await?;
    Ok(workspace)
}

/// Apply an edit, keeping the workspace slug globally unique.
pub async fn update_workspace(
    repo: &dyn WorkspaceRepository,
    mut workspace: Workspace,
) -> AppResult<Workspace> {
    if workspace.name.trim().is_empty() {
        return Err(AppError::BadRequest("workspace name is required".into()));
    }
    if let Some(existing) = repo.find_by_slug(&workspace.slug).await?
        && existing.id != workspace.id
    {
        return Err(AppError::Conflict(format!(
            "workspace slug {:?} already exists",
            workspace.slug
        )));
    }
    workspace.name = workspace.name.trim().to_string();
    repo.update(&workspace).await?;
    Ok(workspace)
}

/// Delete a workspace and everything owned by it (the repository cascades).
pub async fn delete_workspace(repo: &dyn WorkspaceRepository, id: &WorkspaceId) -> AppResult<()> {
    repo.delete(id).await
}
