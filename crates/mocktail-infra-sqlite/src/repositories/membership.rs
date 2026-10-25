use crate::dto::MembershipDoc;
use crate::executor::{SqlExecutor, SqlValue};
use crate::map;
use async_trait::async_trait;
use mocktail_core::modules::auth::application::MembershipRepository;
use mocktail_core::modules::auth::domain::Membership;
use mocktail_core::shared::error::{AppError, AppResult};
use mocktail_core::shared::ids::{MembershipId, UserId, WorkspaceId};
use serde_json::Value;
use std::sync::Arc;

pub struct MembershipRepositoryImpl {
    exec: Arc<dyn SqlExecutor>,
}

impl MembershipRepositoryImpl {
    pub fn new(exec: Arc<dyn SqlExecutor>) -> Self {
        Self { exec }
    }

    async fn query(&self, sql: &str, params: Vec<SqlValue>) -> AppResult<Vec<Value>> {
        self.exec
            .query(sql, params)
            .await
            .map_err(AppError::Internal)
    }

    async fn execute(&self, sql: &str, params: Vec<SqlValue>) -> AppResult<()> {
        self.exec
            .execute(sql, params)
            .await
            .map_err(AppError::Internal)
    }

    fn membership_row(r: &Value) -> AppResult<Membership> {
        let s = |k: &str| {
            r.get(k)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| AppError::Internal(format!("membership row missing {k}")))
        };
        let doc = MembershipDoc {
            id: s("id")?,
            user_id: s("user_id")?,
            workspace_id: s("workspace_id")?,
            role: s("role")?,
        };
        map::membership_from_doc(doc)
    }
}

#[async_trait]
impl MembershipRepository for MembershipRepositoryImpl {
    async fn find(&self, user: &UserId, workspace: &WorkspaceId) -> AppResult<Option<Membership>> {
        let rows = self
            .query(
                "SELECT id, user_id, workspace_id, role FROM memberships                  WHERE user_id=? AND workspace_id=? LIMIT 1",
                vec![SqlValue::Text(user.as_str().to_string()), SqlValue::Text(workspace.as_str().to_string())],
            )
            .await?;
        match rows.first() {
            Some(r) => Ok(Some(Self::membership_row(r)?)),
            None => Ok(None),
        }
    }
    async fn list_for_user(&self, user: &UserId) -> AppResult<Vec<Membership>> {
        let rows = self
            .query(
                "SELECT id, user_id, workspace_id, role FROM memberships WHERE user_id=?",
                vec![SqlValue::Text(user.as_str().to_string())],
            )
            .await?;
        rows.iter().map(Self::membership_row).collect()
    }
    async fn list_for_workspace(&self, workspace: &WorkspaceId) -> AppResult<Vec<Membership>> {
        let rows = self
            .query(
                "SELECT id, user_id, workspace_id, role FROM memberships WHERE workspace_id=?",
                vec![SqlValue::Text(workspace.as_str().to_string())],
            )
            .await?;
        rows.iter().map(Self::membership_row).collect()
    }
    async fn insert(&self, m: &Membership) -> AppResult<()> {
        let doc = map::membership_to_doc(m);
        self.execute(
            "INSERT INTO memberships(id,user_id,workspace_id,role) VALUES(?,?,?,?)",
            vec![
                SqlValue::Text(doc.id),
                SqlValue::Text(doc.user_id),
                SqlValue::Text(doc.workspace_id),
                SqlValue::Text(doc.role),
            ],
        )
        .await
    }
    async fn update(&self, m: &Membership) -> AppResult<()> {
        let doc = map::membership_to_doc(m);
        self.execute(
            "UPDATE memberships SET role=? WHERE id=?",
            vec![SqlValue::Text(doc.role), SqlValue::Text(doc.id)],
        )
        .await
    }
    async fn delete(&self, id: &MembershipId) -> AppResult<()> {
        self.execute(
            "DELETE FROM memberships WHERE id=?",
            vec![SqlValue::Text(id.as_str().to_string())],
        )
        .await
    }
}
