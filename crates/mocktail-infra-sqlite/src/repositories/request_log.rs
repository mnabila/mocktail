use crate::dto::RequestLogDoc;
use crate::executor::{SqlExecutor, SqlValue};
use crate::map;
use async_trait::async_trait;
use mocktail_core::modules::request_log::application::{LogQuery, RequestLogRepository};
use mocktail_core::modules::request_log::domain::{RequestId, RequestLog};
use mocktail_core::shared::error::{AppError, AppResult};
use mocktail_core::shared::ids::{CollectionId, WorkspaceId};
use serde_json::Value;
use std::sync::Arc;

pub struct RequestLogRepositoryImpl {
    exec: Arc<dyn SqlExecutor>,
}

impl RequestLogRepositoryImpl {
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
impl RequestLogRepository for RequestLogRepositoryImpl {
    async fn append(&self, log: &RequestLog) -> AppResult<()> {
        let doc = map::log_to_doc(log);
        let request_headers_json = serde_json::to_string(&doc.request_headers)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let response_json =
            serde_json::to_string(&doc.response).map_err(|e| AppError::Internal(e.to_string()))?;
        self.execute(
            "INSERT INTO request_logs(id,request_id,workspace_id,collection_id,received_at,method,path,requestHeaders,requestBody,matchedMockId,matchReason,status,response,ip,country,lat,lng) \
             VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            vec![
                SqlValue::Text(doc.id),
                SqlValue::Text(doc.request_id),
                SqlValue::Text(doc.workspace_id),
                doc.collection_id.clone().map(SqlValue::Text).unwrap_or(SqlValue::Null),
                SqlValue::I64(doc.received_at),
                SqlValue::Text(doc.method),
                SqlValue::Text(doc.path),
                SqlValue::Text(request_headers_json),
                SqlValue::Text(doc.request_body),
                doc.matched_mock_id.clone().map(SqlValue::Text).unwrap_or(SqlValue::Null),
                doc.match_reason.clone().map(SqlValue::Text).unwrap_or(SqlValue::Null),
                SqlValue::I64(doc.status as i64),
                SqlValue::Text(response_json),
                doc.ip.clone().map(SqlValue::Text).unwrap_or(SqlValue::Null),
                doc.country.clone().map(SqlValue::Text).unwrap_or(SqlValue::Null),
                doc.lat.map(SqlValue::Real).unwrap_or(SqlValue::Null),
                doc.lng.map(SqlValue::Real).unwrap_or(SqlValue::Null),
            ],
        )
        .await
    }
    async fn find_by_request_id(
        &self,
        ws: &WorkspaceId,
        request_id: &RequestId,
    ) -> AppResult<Option<RequestLog>> {
        let rows = self
            .query(
                "SELECT id, request_id, workspace_id, collection_id, received_at, method, path, requestheaders AS requestHeaders, requestbody AS requestBody, matchedmockid AS matchedMockId, matchreason AS matchReason, status, response, ip, country, lat, lng FROM request_logs WHERE workspace_id=? AND request_id=? LIMIT 1",
                vec![
                    SqlValue::Text(ws.as_str().to_string()),
                    SqlValue::Text(request_id.as_str().to_string()),
                ],
            )
            .await?;
        match rows.first() {
            Some(r) => {
                let doc = RequestLogDoc {
                    id: r["id"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing id".into()))?
                        .to_string(),
                    request_id: r["request_id"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing request_id".into()))?
                        .to_string(),
                    workspace_id: r["workspace_id"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing workspace_id".into()))?
                        .to_string(),
                    collection_id: r["collection_id"].as_str().map(|s| s.to_string()),
                    received_at: r["received_at"]
                        .as_i64()
                        .ok_or_else(|| AppError::Internal("missing received_at".into()))?,
                    method: r["method"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing method".into()))?
                        .to_string(),
                    path: r["path"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing path".into()))?
                        .to_string(),
                    request_headers: serde_json::from_str(
                        r["requestHeaders"]
                            .as_str()
                            .ok_or_else(|| AppError::Internal("missing requestHeaders".into()))?,
                    )
                    .map_err(|e| AppError::Internal(e.to_string()))?,
                    request_body: r["requestBody"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing requestBody".into()))?
                        .to_string(),
                    matched_mock_id: r["matchedMockId"].as_str().map(|s| s.to_string()),
                    match_reason: r["matchReason"].as_str().map(|s| s.to_string()),
                    status: r["status"]
                        .as_i64()
                        .ok_or_else(|| AppError::Internal("missing status".into()))?
                        as u16,
                    response: serde_json::from_str(
                        r["response"]
                            .as_str()
                            .ok_or_else(|| AppError::Internal("missing response".into()))?,
                    )
                    .map_err(|e| AppError::Internal(e.to_string()))?,
                    ip: r["ip"].as_str().map(|s| s.to_string()),
                    country: r["country"].as_str().map(|s| s.to_string()),
                    lat: r["lat"].as_f64(),
                    lng: r["lng"].as_f64(),
                };
                Ok(Some(map::log_from_doc(doc)?))
            }
            None => Ok(None),
        }
    }
    async fn recent(
        &self,
        ws: &WorkspaceId,
        coll: Option<&CollectionId>,
        limit: usize,
    ) -> AppResult<Vec<RequestLog>> {
        let (sql, params) = match coll {
            Some(c) => (
                "SELECT id, request_id, workspace_id, collection_id, received_at, method, path, requestheaders AS requestHeaders, requestbody AS requestBody, matchedmockid AS matchedMockId, matchreason AS matchReason, status, response, ip, country, lat, lng FROM request_logs WHERE workspace_id=? AND collection_id=? ORDER BY received_at DESC, id DESC LIMIT ?",
                vec![
                    SqlValue::Text(ws.as_str().to_string()),
                    SqlValue::Text(c.as_str().to_string()),
                    SqlValue::I64(limit as i64),
                ],
            ),
            None => (
                "SELECT id, request_id, workspace_id, collection_id, received_at, method, path, requestheaders AS requestHeaders, requestbody AS requestBody, matchedmockid AS matchedMockId, matchreason AS matchReason, status, response, ip, country, lat, lng FROM request_logs WHERE workspace_id=? ORDER BY received_at DESC LIMIT ?",
                vec![
                    SqlValue::Text(ws.as_str().to_string()),
                    SqlValue::I64(limit as i64),
                ],
            ),
        };
        let rows = self.query(sql, params).await?;
        let mut logs = Vec::new();
        for r in rows {
            let doc = RequestLogDoc {
                id: r["id"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing id".into()))?
                    .to_string(),
                request_id: r["request_id"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing request_id".into()))?
                    .to_string(),
                workspace_id: r["workspace_id"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing workspace_id".into()))?
                    .to_string(),
                collection_id: r["collection_id"].as_str().map(|s| s.to_string()),
                received_at: r["received_at"]
                    .as_i64()
                    .ok_or_else(|| AppError::Internal("missing received_at".into()))?,
                method: r["method"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing method".into()))?
                    .to_string(),
                path: r["path"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing path".into()))?
                    .to_string(),
                request_headers: serde_json::from_str(
                    r["requestHeaders"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing requestHeaders".into()))?,
                )
                .map_err(|e| AppError::Internal(e.to_string()))?,
                request_body: r["requestBody"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing requestBody".into()))?
                    .to_string(),
                matched_mock_id: r["matchedMockId"].as_str().map(|s| s.to_string()),
                match_reason: r["matchReason"].as_str().map(|s| s.to_string()),
                status: r["status"]
                    .as_i64()
                    .ok_or_else(|| AppError::Internal("missing status".into()))?
                    as u16,
                response: serde_json::from_str(
                    r["response"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing response".into()))?,
                )
                .map_err(|e| AppError::Internal(e.to_string()))?,
                ip: r["ip"].as_str().map(|s| s.to_string()),
                country: r["country"].as_str().map(|s| s.to_string()),
                lat: r["lat"].as_f64(),
                lng: r["lng"].as_f64(),
            };
            logs.push(map::log_from_doc(doc)?);
        }
        Ok(logs)
    }
    async fn page(&self, ws: &WorkspaceId, q: &LogQuery<'_>) -> AppResult<Vec<RequestLog>> {
        let mut sql = String::from(
            "SELECT id, request_id, workspace_id, collection_id, received_at, method, path, requestheaders AS requestHeaders, requestbody AS requestBody, matchedmockid AS matchedMockId, matchreason AS matchReason, status, response, ip, country, lat, lng FROM request_logs WHERE workspace_id=?",
        );
        let mut params: Vec<SqlValue> = vec![SqlValue::Text(ws.as_str().to_string())];
        if let Some(c) = q.coll {
            sql.push_str(" AND collection_id=?");
            params.push(SqlValue::Text(c.as_str().to_string()));
        }
        if let Some(p) = q.path_contains.filter(|s| !s.is_empty()) {
            let escaped = p
                .replace('\\', "\\\\")
                .replace('%', r"\%")
                .replace('_', r"\_");
            sql.push_str(" AND path LIKE ? ESCAPE '\\'");
            params.push(SqlValue::Text(format!("%{escaped}%")));
        }
        if let Some(s) = q.status {
            sql.push_str(" AND status=?");
            params.push(SqlValue::I64(s as i64));
        } else if let Some(c) = q.status_class {
            sql.push_str(" AND status>=? AND status<?");
            params.push(SqlValue::I64(c as i64));
            params.push(SqlValue::I64((c + 100) as i64));
        }
        if let Some(cur) = &q.cursor {
            sql.push_str(" AND (received_at < ? OR (received_at = ? AND id < ?))");
            params.push(SqlValue::I64(cur.received_at));
            params.push(SqlValue::I64(cur.received_at));
            params.push(SqlValue::Text(cur.id.clone()));
        }
        sql.push_str(" ORDER BY received_at DESC, id DESC LIMIT ?");
        params.push(SqlValue::I64(q.limit as i64));
        let rows = self.query(&sql, params).await?;
        let mut logs = Vec::new();
        for r in rows {
            let doc = RequestLogDoc {
                id: r["id"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing id".into()))?
                    .to_string(),
                request_id: r["request_id"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing request_id".into()))?
                    .to_string(),
                workspace_id: r["workspace_id"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing workspace_id".into()))?
                    .to_string(),
                collection_id: r["collection_id"].as_str().map(|s| s.to_string()),
                received_at: r["received_at"]
                    .as_i64()
                    .ok_or_else(|| AppError::Internal("missing received_at".into()))?,
                method: r["method"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing method".into()))?
                    .to_string(),
                path: r["path"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing path".into()))?
                    .to_string(),
                request_headers: serde_json::from_str(
                    r["requestHeaders"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing requestHeaders".into()))?,
                )
                .map_err(|e| AppError::Internal(e.to_string()))?,
                request_body: r["requestBody"]
                    .as_str()
                    .ok_or_else(|| AppError::Internal("missing requestBody".into()))?
                    .to_string(),
                matched_mock_id: r["matchedMockId"].as_str().map(|s| s.to_string()),
                match_reason: r["matchReason"].as_str().map(|s| s.to_string()),
                status: r["status"]
                    .as_i64()
                    .ok_or_else(|| AppError::Internal("missing status".into()))?
                    as u16,
                response: serde_json::from_str(
                    r["response"]
                        .as_str()
                        .ok_or_else(|| AppError::Internal("missing response".into()))?,
                )
                .map_err(|e| AppError::Internal(e.to_string()))?,
                ip: r["ip"].as_str().map(|s| s.to_string()),
                country: r["country"].as_str().map(|s| s.to_string()),
                lat: r["lat"].as_f64(),
                lng: r["lng"].as_f64(),
            };
            logs.push(map::log_from_doc(doc)?);
        }
        Ok(logs)
    }
}
