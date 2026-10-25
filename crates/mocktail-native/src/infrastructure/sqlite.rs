//! Native sqlx `SqlExecutor` backing the SQLite repository layer.
//!
//! The `mocktail-infra-sqlite` crate implements every repository port over the
//! generic `SqlExecutor` trait; this wraps a file-backed sqlx SQLite pool so the
//! native binary runs the same store as the Cloudflare Worker's D1 backend. Rows
//! come back as JSON objects keyed by column name (the `SqliteRepositories` SELECTs
//! only read TEXT columns: the `doc` blob and membership scalars), matching the
//! contract the Worker's D1 executor and the sqlx contract test provide.

use anyhow::{Context, Result};
use async_trait::async_trait;
use mocktail_infra_sqlite::{SqlExecutor, SqlValue};
use serde_json::{Map, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Column, Row, SqlitePool};
use std::str::FromStr;
use std::time::Duration;

pub struct SqliteExecutor {
    pool: SqlitePool,
}

impl SqliteExecutor {
    /// Open (creating if absent) the SQLite database at `path`. A single pooled
    /// connection serializes access, which keeps the current-thread runtime free
    /// of `database is locked` races; WAL + a busy timeout cover the rest.
    pub async fn connect(path: &str) -> Result<Self> {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{path}"))
            .with_context(|| format!("parsing sqlite path {path}"))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .with_context(|| format!("opening sqlite database {path}"))?;
        Ok(SqliteExecutor { pool })
    }
}

fn bind<'q>(
    mut q: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    params: Vec<SqlValue>,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    for p in params {
        q = match p {
            SqlValue::Text(s) => q.bind(s),
            SqlValue::I64(n) => q.bind(n),
            SqlValue::Bool(b) => q.bind(b),
            SqlValue::Real(f) => q.bind(f),
            SqlValue::Null => q.bind(Option::<String>::None),
        };
    }
    q
}

/// Decode one column into JSON without knowing its declared type up front.
/// SQLite columns are dynamically typed, so we probe the common storage classes
/// in order (null, integer, real, text) and fall back to Null. Reading an
/// integer column (e.g. `createdAt`) as a String fails, so a single typed
/// `try_get::<String>` is not enough.
fn decode_col(row: &sqlx::sqlite::SqliteRow, idx: usize) -> Value {
    if let Ok(v) = row.try_get::<Option<i64>, _>(idx) {
        return v.map(Value::from).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(idx) {
        return v.map(Value::from).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<String>, _>(idx) {
        return v.map(Value::from).unwrap_or(Value::Null);
    }
    Value::Null
}

#[async_trait]
impl SqlExecutor for SqliteExecutor {
    async fn query(
        &self,
        sql: &str,
        params: Vec<SqlValue>,
    ) -> std::result::Result<Vec<Value>, String> {
        let q = bind(sqlx::query(sql), params);
        let rows = q.fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        let mut out = Vec::with_capacity(rows.len());
        for row in &rows {
            let mut obj = Map::new();
            for col in row.columns() {
                obj.insert(col.name().to_string(), decode_col(row, col.ordinal()));
            }
            out.push(Value::Object(obj));
        }
        Ok(out)
    }

    async fn execute(&self, sql: &str, params: Vec<SqlValue>) -> std::result::Result<(), String> {
        let q = bind(sqlx::query(sql), params);
        q.execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}
