//! Mock use cases (CRUD) + repository port.

use super::domain::Mock;
use crate::shared::error::{AppError, AppResult};
use crate::shared::ids::{CollectionId, MockId, WorkspaceId};
use async_trait::async_trait;

#[async_trait]
pub trait MockRepository: Send + Sync {
    /// All active mocks for a collection (the matcher input).
    async fn list_active(&self, ws: &WorkspaceId, coll: &CollectionId) -> AppResult<Vec<Mock>>;
    async fn find_by_id(&self, id: &MockId) -> AppResult<Option<Mock>>;
    async fn upsert(&self, mock: &Mock) -> AppResult<()>;
    async fn delete(&self, id: &MockId) -> AppResult<()>;
}

pub async fn get_mock(repo: &dyn MockRepository, id: &MockId) -> AppResult<Mock> {
    repo.find_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("mock {id}")))
}

/// Validate and persist a mock (create or update).
pub async fn save_mock(repo: &dyn MockRepository, mock: &Mock) -> AppResult<()> {
    if !mock.path_template.starts_with('/') {
        return Err(AppError::BadRequest(
            "path template must start with '/'".into(),
        ));
    }
    if !(100..=599).contains(&mock.response.status) {
        return Err(AppError::BadRequest("response status out of range".into()));
    }
    if let Some(rl) = &mock.rate_limit {
        if rl.limit == 0 {
            return Err(AppError::BadRequest(
                "rate limit must be greater than zero".into(),
            ));
        }
        if rl.window_secs == 0 {
            return Err(AppError::BadRequest(
                "rate limit window must be greater than zero".into(),
            ));
        }
        if !(400..=599).contains(&rl.status) {
            return Err(AppError::BadRequest(
                "rate limit reject status must be 400-599".into(),
            ));
        }
    }
    repo.upsert(mock).await
}

pub async fn delete_mock(repo: &dyn MockRepository, id: &MockId) -> AppResult<()> {
    repo.delete(id).await
}
