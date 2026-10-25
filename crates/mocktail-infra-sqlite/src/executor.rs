use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, error, span, Level};

/// A bound SQL parameter value.
#[derive(Clone, Debug)]
pub enum SqlValue {
    Text(String),
    I64(i64),
    Real(f64),
    Bool(bool),
    Null,
}

/// Minimal async SQL execution port. Rows are returned as JSON objects keyed by
/// column name; `execute` is for statements with no result set.
#[async_trait]
pub trait SqlExecutor: Send + Sync {
    async fn query(&self, sql: &str, params: Vec<SqlValue>) -> Result<Vec<Value>, String>;
    async fn execute(&self, sql: &str, params: Vec<SqlValue>) -> Result<(), String>;
}

/// Decorator that adds structured logging to any `SqlExecutor`.
pub struct LoggingExecutor {
    inner: Arc<dyn SqlExecutor>,
}

impl LoggingExecutor {
    pub fn new(inner: Arc<dyn SqlExecutor>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl SqlExecutor for LoggingExecutor {
    async fn query(&self, sql: &str, params: Vec<SqlValue>) -> Result<Vec<Value>, String> {
        let span = span!(Level::DEBUG, "sql_query", sql = %sql);
        let _enter = span.enter();

        match self.inner.query(sql, params).await {
            Ok(res) => {
                debug!(count = res.len(), "query successful");
                Ok(res)
            }
            Err(e) => {
                error!(error = %e, "query failed");
                Err(e)
            }
        }
    }

    async fn execute(&self, sql: &str, params: Vec<SqlValue>) -> Result<(), String> {
        let span = span!(Level::DEBUG, "sql_execute", sql = %sql);
        let _enter = span.enter();

        match self.inner.execute(sql, params).await {
            Ok(_) => {
                debug!("execute successful");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, "execute failed");
                Err(e)
            }
        }
    }
}
