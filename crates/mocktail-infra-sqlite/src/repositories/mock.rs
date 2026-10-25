use crate::dto::MockDoc;
use crate::executor::{SqlExecutor, SqlValue};
use crate::map;
use async_trait::async_trait;
use mocktail_core::modules::mock::application::MockRepository;
use mocktail_core::modules::mock::domain::Mock;
use mocktail_core::shared::error::{AppError, AppResult};
use mocktail_core::shared::ids::{CollectionId, MockId, WorkspaceId};
use serde_json::Value;
use std::sync::Arc;

pub struct MockRepositoryImpl {
    exec: Arc<dyn SqlExecutor>,
}

impl MockRepositoryImpl {
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
impl MockRepository for MockRepositoryImpl {
    async fn list_active(&self, ws: &WorkspaceId, coll: &CollectionId) -> AppResult<Vec<Mock>> {
        let rows = self
            .query(
                "SELECT id, workspace_id, collection_id, folder_id, method, pathTemplate, matchers, response, priority, delayMs, fault, rateLimit, created_at, updated_at FROM mocks WHERE workspace_id=? AND collection_id=?",
                vec![
                    SqlValue::Text(ws.as_str().to_string()),
                    SqlValue::Text(coll.as_str().to_string()),
                ],
            )
            .await?;
        let mut mocks = Vec::new();
        for r in rows {
            let doc = MockDoc {
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
                folder_id: r["folder_id"].as_str().map(|s| s.to_string()),
                method: r["method"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing method".into()))?
                    .to_string(),
                path_template: r["pathTemplate"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing pathTemplate".into()))?
                    .to_string(),
                matchers: serde_json::from_str(
                    r["matchers"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing matchers".into()))?,
                )
                .map_err(|e| AppError::Internal(e.to_string()))?,
                response: serde_json::from_str(
                    r["response"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing response".into()))?,
                )
                .map_err(|e| AppError::Internal(e.to_string()))?,
                priority: r["priority"]
                    .as_i64()
                    .ok_or_else(|| AppError::Internal("missing priority".into()))?
                    as i32,
                delay_ms: r["delayMs"]
                    .as_i64()
                    .ok_or_else(|| AppError::Internal("missing delayMs".into()))?
                    as u32,
                fault: r["fault"].as_str().map(|s| s.to_string()),
                rate_limit: r["rateLimit"]
                    .as_str()
                    .and_then(|s| serde_json::from_str(s).ok()),
                created_at: r["created_at"]
                    .as_i64()
                    .ok_or_else(|| AppError::Internal("missing created_at".into()))?,
                updated_at: r["updated_at"]
                    .as_i64()
                    .ok_or_else(|| AppError::Internal("missing updated_at".into()))?,
            };
            mocks.push(map::mock_from_doc(doc)?);
        }
        Ok(mocks)
    }
    async fn find_by_id(&self, id: &MockId) -> AppResult<Option<Mock>> {
        let rows = self
            .query(
                "SELECT id, workspace_id, collection_id, folder_id, method, pathTemplate, matchers, response, priority, delayMs, fault, rateLimit, created_at, updated_at FROM mocks WHERE id=? LIMIT 1",
                vec![SqlValue::Text(id.as_str().to_string())],
            )
            .await?;
        match rows.first() {
            Some(r) => {
                let doc = MockDoc {
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
                    folder_id: r["folder_id"].as_str().map(|s| s.to_string()),
                    method: r["method"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing method".into()))?
                        .to_string(),
                    path_template: r["pathTemplate"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing pathTemplate".into()))?
                        .to_string(),
                    matchers: serde_json::from_str(
                        r["matchers"]
                            .as_str()
                            .ok_or_else(|| AppError::Internal("missing matchers".into()))?,
                    )
                    .map_err(|e| AppError::Internal(e.to_string()))?,
                    response: serde_json::from_str(
                        r["response"]
                            .as_str()
                            .ok_or_else(|| AppError::Internal("missing response".into()))?,
                    )
                    .map_err(|e| AppError::Internal(e.to_string()))?,
                    priority: r["priority"]
                        .as_i64()
                        .ok_or_else(|| AppError::Internal("missing priority".into()))?
                        as i32,
                    delay_ms: r["delayMs"]
                        .as_i64()
                        .ok_or_else(|| AppError::Internal("missing delayMs".into()))?
                        as u32,
                    fault: r["fault"].as_str().map(|s| s.to_string()),
                    rate_limit: r["rateLimit"]
                        .as_str()
                        .and_then(|s| serde_json::from_str(s).ok()),
                    created_at: r["created_at"]
                        .as_i64()
                        .ok_or_else(|| AppError::Internal("missing created_at".into()))?,
                    updated_at: r["updated_at"]
                        .as_i64()
                        .ok_or_else(|| AppError::Internal("missing updated_at".into()))?,
                };
                Ok(Some(map::mock_from_doc(doc)?))
            }
            None => Ok(None),
        }
    }
    async fn upsert(&self, mock: &Mock) -> AppResult<()> {
        let doc = map::mock_to_doc(mock);
        let matchers_json =
            serde_json::to_string(&doc.matchers).map_err(|e| AppError::Internal(e.to_string()))?;
        let response_json =
            serde_json::to_string(&doc.response).map_err(|e| AppError::Internal(e.to_string()))?;
        let rate_limit_json = doc
            .rate_limit
            .as_ref()
            .map(|rl| serde_json::to_string(rl).map_err(|e| AppError::Internal(e.to_string())))
            .transpose()?;

        self.execute(
            "INSERT INTO mocks(id,workspace_id,collection_id,folder_id,method,path_template,matchers,response,priority,delay_ms,fault,rate_limit,created_at,updated_at) \
             VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?) \
             ON CONFLICT(id) DO UPDATE SET \
             workspace_id=excluded.workspace_id, collection_id=excluded.collection_id, folder_id=excluded.folder_id, \
             method=excluded.method, path_template=excluded.path_template, matchers=excluded.matchers, response=excluded.response, \
             priority=excluded.priority, delay_ms=excluded.delay_ms, fault=excluded.fault, rate_limit=excluded.rate_limit, \
             updated_at=excluded.updated_at",
            vec![
                SqlValue::Text(doc.id),
                SqlValue::Text(doc.workspace_id),
                SqlValue::Text(doc.collection_id),
                doc.folder_id.map(SqlValue::Text).unwrap_or(SqlValue::Null),
                SqlValue::Text(doc.method),
                SqlValue::Text(doc.path_template),
                SqlValue::Text(matchers_json),
                SqlValue::Text(response_json),
                SqlValue::I64(doc.priority as i64),
                SqlValue::I64(doc.delay_ms as i64),
                doc.fault.map(SqlValue::Text).unwrap_or(SqlValue::Null),
                rate_limit_json.map(SqlValue::Text).unwrap_or(SqlValue::Null),
                SqlValue::I64(doc.created_at),
                SqlValue::I64(doc.updated_at),
            ],
        )
        .await
    }
    async fn delete(&self, id: &MockId) -> AppResult<()> {
        self.execute(
            "DELETE FROM mocks WHERE id=?",
            vec![SqlValue::Text(id.as_str().to_string())],
        )
        .await
    }
}
