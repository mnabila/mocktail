use crate::dto::CollectionDoc;
use crate::executor::{SqlExecutor, SqlValue};
use crate::map;
use async_trait::async_trait;
use mocktail_core::modules::collection::application::CollectionRepository;
use mocktail_core::modules::collection::domain::Collection;
use mocktail_core::shared::error::{AppError, AppResult};
use mocktail_core::shared::ids::{CollectionId, Slug, WorkspaceId};
use serde_json::Value;
use std::sync::Arc;

pub struct CollectionRepositoryImpl {
    exec: Arc<dyn SqlExecutor>,
}

impl CollectionRepositoryImpl {
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
impl CollectionRepository for CollectionRepositoryImpl {
    async fn find_by_slug(&self, ws: &WorkspaceId, slug: &Slug) -> AppResult<Option<Collection>> {
        let rows = self
            .query(
                "SELECT id, workspace_id, slug, name, description, source, base_path, cors, version, created_at FROM collections WHERE workspace_id=? AND slug=? LIMIT 1",
                vec![
                    SqlValue::Text(ws.as_str().to_string()),
                    SqlValue::Text(slug.as_str().to_string()),
                ],
            )
            .await?;
        match rows.first() {
            Some(r) => {
                let doc = CollectionDoc {
                    id: r["id"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing id".into()))?
                        .to_string(),
                    workspace_id: r["workspace_id"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing workspace_id".into()))?
                        .to_string(),
                    name: r["name"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing name".into()))?
                        .to_string(),
                    slug: r["slug"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing slug".into()))?
                        .to_string(),
                    description: r["description"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing description".into()))?
                        .to_string(),
                    source: r["source"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing source".into()))?
                        .to_string(),
                    base_path: r["base_path"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing base_path".into()))?
                        .to_string(),
                    cors: serde_json::from_str(
                        r["cors"]
                            .as_str()
                            .ok_or_else(|| AppError::Internal("missing cors".into()))?,
                    )
                    .map_err(|e| AppError::Internal(e.to_string()))?,
                    version: r["version"]
                        .as_i64()
                        .ok_or_else(|| AppError::Internal("missing version".into()))?,
                    created_at: r["created_at"]
                        .as_i64()
                        .ok_or_else(|| AppError::Internal("missing created_at".into()))?,
                };
                Ok(Some(map::collection_from_doc(doc)?))
            }
            None => Ok(None),
        }
    }
    async fn find_by_id(&self, id: &CollectionId) -> AppResult<Option<Collection>> {
        let rows = self
            .query(
                "SELECT id, workspace_id, slug, name, description, source, base_path, cors, version, created_at FROM collections WHERE id=? LIMIT 1",
                vec![SqlValue::Text(id.as_str().to_string())],
            )
            .await?;
        match rows.first() {
            Some(r) => {
                let doc = CollectionDoc {
                    id: r["id"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing id".into()))?
                        .to_string(),
                    workspace_id: r["workspace_id"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing workspace_id".into()))?
                        .to_string(),
                    name: r["name"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing name".into()))?
                        .to_string(),
                    slug: r["slug"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing slug".into()))?
                        .to_string(),
                    description: r["description"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing description".into()))?
                        .to_string(),
                    source: r["source"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing source".into()))?
                        .to_string(),
                    base_path: r["base_path"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing base_path".into()))?
                        .to_string(),
                    cors: serde_json::from_str(
                        r["cors"]
                            .as_str()
                            .ok_or_else(|| AppError::Internal("missing cors".into()))?,
                    )
                    .map_err(|e| AppError::Internal(e.to_string()))?,
                    version: r["version"]
                        .as_i64()
                        .ok_or_else(|| AppError::Internal("missing version".into()))?,
                    created_at: r["created_at"]
                        .as_i64()
                        .ok_or_else(|| AppError::Internal("missing created_at".into()))?,
                };
                Ok(Some(map::collection_from_doc(doc)?))
            }
            None => Ok(None),
        }
    }
    async fn list(&self, ws: &WorkspaceId) -> AppResult<Vec<Collection>> {
        let rows = self
            .query(
                "SELECT id, workspace_id, slug, name, description, source, base_path, cors, version, created_at FROM collections WHERE workspace_id=?",
                vec![SqlValue::Text(ws.as_str().to_string())],
            )
            .await?;
        let mut collections = Vec::new();
        for r in rows {
            let doc = CollectionDoc {
                id: r["id"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing id".into()))?
                    .to_string(),
                workspace_id: r["workspace_id"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing workspace_id".into()))?
                    .to_string(),
                name: r["name"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing name".into()))?
                    .to_string(),
                slug: r["slug"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing slug".into()))?
                    .to_string(),
                description: r["description"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing description".into()))?
                    .to_string(),
                source: r["source"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing source".into()))?
                    .to_string(),
                base_path: r["base_path"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing base_path".into()))?
                    .to_string(),
                cors: serde_json::from_str(
                    r["cors"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing cors".into()))?,
                )
                .map_err(|e| AppError::Internal(e.to_string()))?,
                version: r["version"]
                    .as_i64()
                    .ok_or_else(|| AppError::Internal("missing version".into()))?,
                created_at: r["created_at"]
                    .as_i64()
                    .ok_or_else(|| AppError::Internal("missing created_at".into()))?,
            };
            collections.push(map::collection_from_doc(doc)?);
        }
        Ok(collections)
    }
    async fn insert(&self, c: &Collection) -> AppResult<()> {
        let doc = map::collection_to_doc(c);
        let cors_json =
            serde_json::to_string(&doc.cors).map_err(|e| AppError::Internal(e.to_string()))?;
        self.execute(
            "INSERT INTO collections(id,workspace_id,slug,name,description,source,base_path,cors,version,created_at) VALUES(?,?,?,?,?,?,?,?,?,?)",
            vec![
                SqlValue::Text(doc.id),
                SqlValue::Text(doc.workspace_id),
                SqlValue::Text(doc.slug),
                SqlValue::Text(doc.name),
                SqlValue::Text(doc.description),
                SqlValue::Text(doc.source),
                SqlValue::Text(doc.base_path),
                SqlValue::Text(cors_json),
                SqlValue::I64(doc.version),
                SqlValue::I64(doc.created_at),
            ],
        )
        .await
    }
    async fn update(&self, c: &Collection) -> AppResult<()> {
        let doc = map::collection_to_doc(c);
        let cors_json =
            serde_json::to_string(&doc.cors).map_err(|e| AppError::Internal(e.to_string()))?;
        self.execute(
            "UPDATE collections SET slug=?, name=?, description=?, source=?, base_path=?, cors=?, version=?, created_at=? WHERE id=?",
            vec![
                SqlValue::Text(doc.slug),
                SqlValue::Text(doc.name),
                SqlValue::Text(doc.description),
                SqlValue::Text(doc.source),
                SqlValue::Text(doc.base_path),
                SqlValue::Text(cors_json),
                SqlValue::I64(doc.version),
                SqlValue::I64(doc.created_at),
                SqlValue::Text(doc.id),
            ],
        )
        .await
    }
    async fn delete(&self, id: &CollectionId) -> AppResult<()> {
        let id_str = id.as_str();
        self.execute(
            "DELETE FROM mocks WHERE collection_id=?",
            vec![SqlValue::Text(id_str.to_string())],
        )
        .await?;
        self.execute(
            "DELETE FROM folders WHERE collection_id=?",
            vec![SqlValue::Text(id_str.to_string())],
        )
        .await?;
        self.execute(
            "DELETE FROM collections WHERE id=?",
            vec![SqlValue::Text(id_str.to_string())],
        )
        .await
    }
}
