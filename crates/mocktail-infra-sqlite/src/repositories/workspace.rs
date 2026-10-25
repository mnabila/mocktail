use crate::dto::WorkspaceDoc;
use crate::executor::{SqlExecutor, SqlValue};
use crate::map;
use async_trait::async_trait;
use mocktail_core::modules::workspace::application::WorkspaceRepository;
use mocktail_core::modules::workspace::domain::Workspace;
use mocktail_core::shared::error::{AppError, AppResult};
use mocktail_core::shared::ids::{Slug, WorkspaceId};
use serde_json::Value;
use std::sync::Arc;

pub struct WorkspaceRepositoryImpl {
    exec: Arc<dyn SqlExecutor>,
}

impl WorkspaceRepositoryImpl {
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
}

#[async_trait]
impl WorkspaceRepository for WorkspaceRepositoryImpl {
    async fn find_by_slug(&self, slug: &Slug) -> AppResult<Option<Workspace>> {
        let rows = self
            .query(
                "SELECT id, name, slug, created_at FROM workspaces WHERE slug=? LIMIT 1",
                vec![SqlValue::Text(slug.as_str().to_string())],
            )
            .await?;
        match rows.first() {
            Some(r) => {
                let doc = WorkspaceDoc {
                    id: r["id"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing id".into()))?
                        .to_string(),
                    name: r["name"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing name".into()))?
                        .to_string(),
                    slug: r["slug"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing slug".into()))?
                        .to_string(),
                    created_at: r["created_at"]
                        .as_i64()
                        .ok_or_else(|| AppError::Internal("missing created_at".into()))?,
                };
                Ok(Some(map::workspace_from_doc(doc)?))
            }
            None => Ok(None),
        }
    }
    async fn find_by_id(&self, id: &WorkspaceId) -> AppResult<Option<Workspace>> {
        let rows = self
            .query(
                "SELECT id, name, slug, created_at FROM workspaces WHERE id=? LIMIT 1",
                vec![SqlValue::Text(id.as_str().to_string())],
            )
            .await?;
        match rows.first() {
            Some(r) => {
                let doc = WorkspaceDoc {
                    id: r["id"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing id".into()))?
                        .to_string(),
                    name: r["name"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing name".into()))?
                        .to_string(),
                    slug: r["slug"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing slug".into()))?
                        .to_string(),
                    created_at: r["created_at"]
                        .as_i64()
                        .ok_or_else(|| AppError::Internal("missing created_at".into()))?,
                };
                Ok(Some(map::workspace_from_doc(doc)?))
            }
            None => Ok(None),
        }
    }
    async fn insert(&self, workspace: &Workspace) -> AppResult<()> {
        let doc = map::workspace_to_doc(workspace);
        self.execute(
            "INSERT INTO workspaces(id,slug,name,created_at) VALUES(?,?,?,?)",
            vec![
                SqlValue::Text(doc.id),
                SqlValue::Text(doc.slug),
                SqlValue::Text(doc.name),
                SqlValue::I64(doc.created_at),
            ],
        )
        .await
    }
    async fn update(&self, workspace: &Workspace) -> AppResult<()> {
        let doc = map::workspace_to_doc(workspace);
        self.execute(
            "UPDATE workspaces SET slug=?, name=?, created_at=? WHERE id=?",
            vec![
                SqlValue::Text(doc.slug),
                SqlValue::Text(doc.name),
                SqlValue::I64(doc.created_at),
                SqlValue::Text(doc.id),
            ],
        )
        .await
    }
    async fn delete(&self, id: &WorkspaceId) -> AppResult<()> {
        let id_str = id.as_str();
        for table in [
            "mocks",
            "folders",
            "collections",
            "request_logs",
            "memberships",
        ] {
            self.execute(
                &format!("DELETE FROM {table} WHERE workspace_id=?"),
                vec![SqlValue::Text(id_str.to_string())],
            )
            .await?;
        }
        self.execute(
            "DELETE FROM workspaces WHERE id=?",
            vec![SqlValue::Text(id_str.to_string())],
        )
        .await
    }
    async fn list(&self) -> AppResult<Vec<Workspace>> {
        let rows = self
            .query("SELECT id, name, slug, created_at FROM workspaces", vec![])
            .await?;
        let mut workspaces = Vec::new();
        for r in rows {
            let doc = WorkspaceDoc {
                id: r["id"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing id".into()))?
                    .to_string(),
                name: r["name"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing name".into()))?
                    .to_string(),
                slug: r["slug"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing slug".into()))?
                    .to_string(),
                created_at: r["created_at"]
                    .as_i64()
                    .ok_or_else(|| AppError::Internal("missing created_at".into()))?,
            };
            workspaces.push(map::workspace_from_doc(doc)?);
        }
        Ok(workspaces)
    }
}
