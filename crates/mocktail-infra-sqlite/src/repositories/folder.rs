use crate::dto::FolderDoc;
use crate::executor::{SqlExecutor, SqlValue};
use crate::map;
use async_trait::async_trait;
use mocktail_core::modules::folder::application::FolderRepository;
use mocktail_core::modules::folder::domain::Folder;
use mocktail_core::shared::error::{AppError, AppResult};
use mocktail_core::shared::ids::{CollectionId, FolderId, WorkspaceId};
use serde_json::Value;
use std::sync::Arc;

pub struct FolderRepositoryImpl {
    exec: Arc<dyn SqlExecutor>,
}

impl FolderRepositoryImpl {
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
impl FolderRepository for FolderRepositoryImpl {
    async fn list(&self, ws: &WorkspaceId, coll: &CollectionId) -> AppResult<Vec<Folder>> {
        let rows = self
            .query(
                "SELECT id, workspace_id, collection_id, name, parentId, created_at FROM folders WHERE workspace_id=? AND collection_id=?",
                vec![
                    SqlValue::Text(ws.as_str().to_string()),
                    SqlValue::Text(coll.as_str().to_string()),
                ],
            )
            .await?;
        let mut folders = Vec::new();
        for r in rows {
            let doc = FolderDoc {
                id: r["id"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing id".into()))?
                    .to_string(),
                workspace_id: r["workspace_id"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing workspace_id".into()))?
                    .to_string(),
                collection_id: r["collection_id"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing collection_id".into()))?
                    .to_string(),
                name: r["name"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing name".into()))?
                    .to_string(),
                parent_id: r["parentId"].as_str().map(|s| s.to_string()),
                created_at: r["created_at"]
                    .as_i64()
                    .ok_or_else(|| AppError::Internal("missing created_at".into()))?,
            };
            folders.push(map::folder_from_doc(doc)?);
        }
        Ok(folders)
    }
    async fn find_by_id(&self, id: &FolderId) -> AppResult<Option<Folder>> {
        let rows = self
            .query(
                "SELECT id, workspace_id, collection_id, name, parentId, created_at FROM folders WHERE id=? LIMIT 1",
                vec![SqlValue::Text(id.as_str().to_string())],
            )
            .await?;
        match rows.first() {
            Some(r) => {
                let doc = FolderDoc {
                    id: r["id"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing id".into()))?
                        .to_string(),
                    workspace_id: r["workspace_id"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing workspace_id".into()))?
                        .to_string(),
                    collection_id: r["collection_id"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing collection_id".into()))?
                        .to_string(),
                    name: r["name"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing name".into()))?
                        .to_string(),
                    parent_id: r["parentId"].as_str().map(|s| s.to_string()),
                    created_at: r["created_at"]
                        .as_i64()
                        .ok_or_else(|| AppError::Internal("missing created_at".into()))?,
                };
                Ok(Some(map::folder_from_doc(doc)?))
            }
            None => Ok(None),
        }
    }
    async fn insert(&self, folder: &Folder) -> AppResult<()> {
        let doc = map::folder_to_doc(folder);
        self.execute(
            "INSERT INTO folders(id,workspace_id,collection_id,name,parentId,created_at) VALUES(?,?,?,?,?,?)",
            vec![
                SqlValue::Text(doc.id),
                SqlValue::Text(doc.workspace_id),
                SqlValue::Text(doc.collection_id),
                SqlValue::Text(doc.name),
                doc.parent_id.map(SqlValue::Text).unwrap_or(SqlValue::Null),
                SqlValue::I64(doc.created_at),
            ],
        )
        .await
    }
    async fn delete(&self, id: &FolderId) -> AppResult<()> {
        self.execute(
            "DELETE FROM folders WHERE id=?",
            vec![SqlValue::Text(id.as_str().to_string())],
        )
        .await
    }
}
