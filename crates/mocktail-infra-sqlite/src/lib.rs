//! SQLite implementation of the repository ports.
//!
//! This crate implements the domain repositories using a JSON-document pattern
//! in SQLite, indexed by scalar columns.

pub mod dto;
pub mod error;
pub mod executor;
pub mod map;
pub mod migrations;
pub mod repositories;

use crate::repositories::{
    collection::CollectionRepositoryImpl, folder::FolderRepositoryImpl,
    membership::MembershipRepositoryImpl, mock::MockRepositoryImpl,
    request_log::RequestLogRepositoryImpl, user::UserRepositoryImpl,
    workspace::WorkspaceRepositoryImpl,
};
pub use executor::{SqlExecutor, SqlValue};
use std::sync::Arc;

// Re-implement traits for the facade to maintain backward compatibility
use async_trait::async_trait;
use mocktail_core::modules::auth::application::{MembershipRepository, UserRepository};
use mocktail_core::modules::auth::domain::{Membership, User};
use mocktail_core::modules::collection::application::CollectionRepository;
use mocktail_core::modules::collection::domain::Collection;
use mocktail_core::modules::folder::application::FolderRepository;
use mocktail_core::modules::folder::domain::Folder;
use mocktail_core::modules::mock::application::MockRepository;
use mocktail_core::modules::mock::domain::Mock;
use mocktail_core::modules::request_log::application::{LogQuery, RequestLogRepository};
use mocktail_core::modules::request_log::domain::{RequestId, RequestLog};
use mocktail_core::modules::workspace::application::WorkspaceRepository;
use mocktail_core::modules::workspace::domain::Workspace;
use mocktail_core::shared::error::AppResult;
use mocktail_core::shared::ids::{
    CollectionId, FolderId, MembershipId, MockId, Slug, UserId, WorkspaceId,
};

/// A facade that aggregates all SQLite repository implementations.
pub struct SqliteRepositories {
    pub workspace: WorkspaceRepositoryImpl,
    pub collection: CollectionRepositoryImpl,
    pub folder: FolderRepositoryImpl,
    pub mock: MockRepositoryImpl,
    pub request_log: RequestLogRepositoryImpl,
    pub user: UserRepositoryImpl,
    pub membership: MembershipRepositoryImpl,
    exec: Arc<dyn SqlExecutor>,
}

impl SqliteRepositories {
    pub fn new(exec: Arc<dyn SqlExecutor>) -> Self {
        let exec = Arc::new(executor::LoggingExecutor::new(exec));
        Self {
            workspace: WorkspaceRepositoryImpl::new(exec.clone()),
            collection: CollectionRepositoryImpl::new(exec.clone()),
            folder: FolderRepositoryImpl::new(exec.clone()),
            mock: MockRepositoryImpl::new(exec.clone()),
            request_log: RequestLogRepositoryImpl::new(exec.clone()),
            user: UserRepositoryImpl::new(exec.clone()),
            membership: MembershipRepositoryImpl::new(exec.clone()),
            exec,
        }
    }

    pub async fn ensure_schema(&self) -> AppResult<()> {
        migrations::run(self.exec.as_ref()).await
    }
}

#[async_trait]
impl WorkspaceRepository for SqliteRepositories {
    async fn find_by_slug(&self, slug: &Slug) -> AppResult<Option<Workspace>> {
        self.workspace.find_by_slug(slug).await
    }
    async fn find_by_id(&self, id: &WorkspaceId) -> AppResult<Option<Workspace>> {
        self.workspace.find_by_id(id).await
    }
    async fn insert(&self, workspace: &Workspace) -> AppResult<()> {
        self.workspace.insert(workspace).await
    }
    async fn update(&self, workspace: &Workspace) -> AppResult<()> {
        self.workspace.update(workspace).await
    }
    async fn delete(&self, id: &WorkspaceId) -> AppResult<()> {
        self.workspace.delete(id).await
    }
    async fn list(&self) -> AppResult<Vec<Workspace>> {
        self.workspace.list().await
    }
}

#[async_trait]
impl CollectionRepository for SqliteRepositories {
    async fn find_by_slug(&self, ws: &WorkspaceId, slug: &Slug) -> AppResult<Option<Collection>> {
        self.collection.find_by_slug(ws, slug).await
    }
    async fn find_by_id(&self, id: &CollectionId) -> AppResult<Option<Collection>> {
        self.collection.find_by_id(id).await
    }
    async fn list(&self, ws: &WorkspaceId) -> AppResult<Vec<Collection>> {
        self.collection.list(ws).await
    }
    async fn insert(&self, c: &Collection) -> AppResult<()> {
        self.collection.insert(c).await
    }
    async fn update(&self, c: &Collection) -> AppResult<()> {
        self.collection.update(c).await
    }
    async fn delete(&self, id: &CollectionId) -> AppResult<()> {
        self.collection.delete(id).await
    }
}

#[async_trait]
impl FolderRepository for SqliteRepositories {
    async fn list(&self, ws: &WorkspaceId, coll: &CollectionId) -> AppResult<Vec<Folder>> {
        self.folder.list(ws, coll).await
    }
    async fn find_by_id(&self, id: &FolderId) -> AppResult<Option<Folder>> {
        self.folder.find_by_id(id).await
    }
    async fn insert(&self, folder: &Folder) -> AppResult<()> {
        self.folder.insert(folder).await
    }
    async fn delete(&self, id: &FolderId) -> AppResult<()> {
        self.folder.delete(id).await
    }
}

#[async_trait]
impl MockRepository for SqliteRepositories {
    async fn list_active(&self, ws: &WorkspaceId, coll: &CollectionId) -> AppResult<Vec<Mock>> {
        self.mock.list_active(ws, coll).await
    }
    async fn find_by_id(&self, id: &MockId) -> AppResult<Option<Mock>> {
        self.mock.find_by_id(id).await
    }
    async fn upsert(&self, mock: &Mock) -> AppResult<()> {
        self.mock.upsert(mock).await
    }
    async fn delete(&self, id: &MockId) -> AppResult<()> {
        self.mock.delete(id).await
    }
}

#[async_trait]
impl RequestLogRepository for SqliteRepositories {
    async fn append(&self, log: &RequestLog) -> AppResult<()> {
        self.request_log.append(log).await
    }
    async fn find_by_request_id(
        &self,
        ws: &WorkspaceId,
        rid: &RequestId,
    ) -> AppResult<Option<RequestLog>> {
        self.request_log.find_by_request_id(ws, rid).await
    }
    async fn recent(
        &self,
        ws: &WorkspaceId,
        coll: Option<&CollectionId>,
        limit: usize,
    ) -> AppResult<Vec<RequestLog>> {
        self.request_log.recent(ws, coll, limit).await
    }
    async fn page(&self, ws: &WorkspaceId, q: &LogQuery<'_>) -> AppResult<Vec<RequestLog>> {
        self.request_log.page(ws, q).await
    }
}

#[async_trait]
impl UserRepository for SqliteRepositories {
    async fn find_by_identity(&self, p: &str, s: &str) -> AppResult<Option<User>> {
        self.user.find_by_identity(p, s).await
    }
    async fn find_by_id(&self, id: &UserId) -> AppResult<Option<User>> {
        self.user.find_by_id(id).await
    }
    async fn find_by_username(&self, username: &str) -> AppResult<Option<User>> {
        self.user.find_by_username(username).await
    }
    async fn insert(&self, user: &User) -> AppResult<()> {
        self.user.insert(user).await
    }
    async fn update(&self, user: &User) -> AppResult<()> {
        self.user.update(user).await
    }
    async fn add_identity(&self, user_id: &UserId, provider: &str, subject: &str, email: Option<String>) -> AppResult<()> {
        self.user.add_identity(user_id, provider, subject, email).await
    }
    async fn find_identities(&self, user_id: &UserId) -> AppResult<Vec<mocktail_core::modules::auth::domain::Identity>> {
        self.user.find_identities(user_id).await
    }
}

#[async_trait]
impl MembershipRepository for SqliteRepositories {
    async fn find(&self, u: &UserId, w: &WorkspaceId) -> AppResult<Option<Membership>> {
        self.membership.find(u, w).await
    }
    async fn list_for_user(&self, u: &UserId) -> AppResult<Vec<Membership>> {
        self.membership.list_for_user(u).await
    }
    async fn list_for_workspace(&self, w: &WorkspaceId) -> AppResult<Vec<Membership>> {
        self.membership.list_for_workspace(w).await
    }
    async fn insert(&self, m: &Membership) -> AppResult<()> {
        self.membership.insert(m).await
    }
    async fn update(&self, m: &Membership) -> AppResult<()> {
        self.membership.update(m).await
    }
    async fn delete(&self, id: &MembershipId) -> AppResult<()> {
        self.membership.delete(id).await
    }
}
