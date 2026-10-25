//! Repository ports owned by the auth use cases.

use crate::modules::auth::domain::{Identity, Membership, User};
use crate::shared::error::AppResult;
use crate::shared::ids::{MembershipId, UserId, WorkspaceId};
use async_trait::async_trait;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_identity(&self, provider: &str, subject: &str) -> AppResult<Option<User>>;
    async fn find_by_id(&self, id: &UserId) -> AppResult<Option<User>>;
    async fn find_by_username(&self, username: &str) -> AppResult<Option<User>>;
    async fn insert(&self, user: &User) -> AppResult<()>;
    async fn update(&self, user: &User) -> AppResult<()>;
    async fn add_identity(&self, user_id: &UserId, provider: &str, subject: &str, email: Option<String>) -> AppResult<()>;
    async fn find_identities(&self, user_id: &UserId) -> AppResult<Vec<Identity>>;
}

#[async_trait]
pub trait MembershipRepository: Send + Sync {
    async fn find(&self, user: &UserId, workspace: &WorkspaceId) -> AppResult<Option<Membership>>;
    async fn list_for_user(&self, user: &UserId) -> AppResult<Vec<Membership>>;
    async fn list_for_workspace(&self, workspace: &WorkspaceId) -> AppResult<Vec<Membership>>;
    async fn insert(&self, membership: &Membership) -> AppResult<()>;
    async fn update(&self, membership: &Membership) -> AppResult<()>;
    async fn delete(&self, id: &MembershipId) -> AppResult<()>;
}
