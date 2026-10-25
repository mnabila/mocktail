//! Embedded SQLite migrations from `migrations/*.sql`.
//!
//! Migration files live in `crates/mocktail-infra-sqlite/migrations/` and are
//! compiled directly into the binary via `rust-embed`, then executed in
//! lexicographical order and tracked in the `_migrations` table.

use crate::{SqlExecutor, SqlValue, error::infra};
use mocktail_core::shared::error::{AppError, AppResult};
use rust_embed::RustEmbed;
use serde_json::Value;
use std::collections::HashSet;

#[derive(RustEmbed)]
#[folder = "migrations/"]
struct EmbeddedMigrations;

/// Return all embedded migrations as `(filename, sql)` sorted by filename.
pub fn all() -> AppResult<Vec<(String, String)>> {
    let mut names: Vec<String> = EmbeddedMigrations::iter()
        .filter(|f| f.ends_with(".sql"))
        .map(|f| f.into_owned())
        .collect();
    names.sort();

    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let file = EmbeddedMigrations::get(&name)
            .ok_or_else(|| AppError::Internal(format!("missing embedded migration {name}")))?;
        let sql = std::str::from_utf8(file.data.as_ref())
            .map_err(infra)?
            .to_string();
        out.push((name, sql));
    }
    Ok(out)
}

/// Apply all pending embedded migrations against `exec`.
pub async fn run(exec: &dyn SqlExecutor) -> AppResult<()> {
    exec.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (\
            name TEXT PRIMARY KEY, \
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP\
        )",
        vec![],
    )
    .await
    .map_err(infra)?;

    let rows = exec
        .query("SELECT name FROM _migrations", vec![])
        .await
        .map_err(infra)?;
    let applied: HashSet<String> = rows
        .iter()
        .filter_map(|r| r.get("name").and_then(Value::as_str).map(str::to_string))
        .collect();

    for (name, sql) in all()? {
        if applied.contains(&name) {
            continue;
        }
        for stmt in statements(&sql) {
            exec.execute(&stmt, vec![])
                .await
                .map_err(|err| infra(format!("migration {name} failed: {err}")))?;
        }
        exec.execute(
            "INSERT INTO _migrations(name) VALUES(?)",
            vec![SqlValue::Text(name)],
        )
        .await
        .map_err(infra)?;
    }

    Ok(())
}

/// Strip `--` line comments and split a migration file into non-empty SQL statements.
fn statements(sql: &str) -> Vec<String> {
    let stripped: String = sql
        .lines()
        .map(|line| match line.find("--") {
            Some(idx) => &line[..idx],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n");

    stripped
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_migrations_are_sorted_and_non_empty() {
        let migrations = all().expect("load embedded migrations");
        let names: Vec<&str> = migrations.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["0001_init.sql"]);
        for (name, sql) in &migrations {
            assert!(
                !statements(sql).is_empty(),
                "migration {name} has no statements"
            );
        }
    }

    #[test]
    fn statements_strips_comments_with_semicolons() {
        let sql = "-- header; with semicolon\nCREATE TABLE t (id TEXT); -- trailing; comment\n";
        assert_eq!(statements(sql), vec!["CREATE TABLE t (id TEXT)"]);
    }
}
