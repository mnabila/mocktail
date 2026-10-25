use crate::executor::{SqlExecutor, SqlValue};
use async_trait::async_trait;
use mocktail_core::modules::auth::application::UserRepository;
use mocktail_core::modules::auth::domain::{Identity, User};
use mocktail_core::shared::error::{AppError, AppResult};
use mocktail_core::shared::ids::UserId;
use serde_json::Value;
use std::sync::Arc;

pub struct UserRepositoryImpl {
    exec: Arc<dyn SqlExecutor>,
}

impl UserRepositoryImpl {
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

    async fn find_identities_impl(&self, user_id: &UserId) -> AppResult<Vec<Identity>> {
        let rows = self
            .query(
                "SELECT provider, subject, email FROM user_identities WHERE user_id=?",
                vec![SqlValue::Text(user_id.to_string())],
            )
            .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let provider = row["provider"].as_str().unwrap_or_default().to_string();
                let subject = row["subject"].as_str().unwrap_or_default().to_string();
                let email = row["email"].as_str().map(|s| s.to_string());
                Identity {
                    provider,
                    subject,
                    email,
                }
            })
            .collect())
    }
}

#[async_trait]
impl UserRepository for UserRepositoryImpl {
    async fn find_by_identity(&self, provider: &str, subject: &str) -> AppResult<Option<User>> {
        let rows = self
            .query(
                "SELECT u.id, u.username, u.display_name, u.created_at FROM users u JOIN user_identities i ON i.user_id = u.id WHERE i.provider=? AND i.subject=? LIMIT 1",
                vec![SqlValue::Text(provider.to_string()),
                    SqlValue::Text(subject.to_string()),
                ],
            )
            .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let row = &rows[0];
        Ok(Some(User {
            id: UserId::new(row["id"].as_str().unwrap_or_default()),
            username: row["username"].as_str().unwrap_or_default().to_string(),
            display_name: row["display_name"].as_str().unwrap_or_default().to_string(),
            created_at: row["created_at"].as_i64().unwrap_or(0),
        }))
    }

    async fn find_by_id(&self, id: &UserId) -> AppResult<Option<User>> {
        let rows = self
            .query(
                "SELECT id, username, display_name, created_at FROM users WHERE id=?",
                vec![SqlValue::Text(id.to_string())],
            )
            .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let row = &rows[0];
        Ok(Some(User {
            id: UserId::new(row["id"].as_str().unwrap_or_default()),
            username: row["username"].as_str().unwrap_or_default().to_string(),
            display_name: row["display_name"].as_str().unwrap_or_default().to_string(),
            created_at: row["created_at"].as_i64().unwrap_or(0),
        }))
    }

    async fn find_by_username(&self, username: &str) -> AppResult<Option<User>> {
        let rows = self
            .query(
                "SELECT id, username, display_name, created_at FROM users WHERE username=? LIMIT 1",
                vec![SqlValue::Text(username.to_string())],
            )
            .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let row = &rows[0];
        Ok(Some(User {
            id: UserId::new(row["id"].as_str().unwrap_or_default()),
            username: row["username"].as_str().unwrap_or_default().to_string(),
            display_name: row["display_name"].as_str().unwrap_or_default().to_string(),
            created_at: row["created_at"].as_i64().unwrap_or(0),
        }))
    }

    async fn insert(&self, user: &User) -> AppResult<()> {
        self.execute(
            "INSERT INTO users(id, username, display_name, created_at) VALUES(?,?,?,?)",
            vec![
                SqlValue::Text(user.id.to_string()),
                SqlValue::Text(user.username.clone()),
                SqlValue::Text(user.display_name.clone()),
                SqlValue::I64(user.created_at),
            ],
        )
        .await
    }

    async fn update(&self, user: &User) -> AppResult<()> {
        self.execute(
            "UPDATE users SET username=?, display_name=? WHERE id=?",
            vec![
                SqlValue::Text(user.username.clone()),
                SqlValue::Text(user.display_name.clone()),
                SqlValue::Text(user.id.to_string()),
            ],
        )
        .await
    }

    async fn add_identity(&self, user_id: &UserId, provider: &str, subject: &str, email: Option<String>) -> AppResult<()> {
        self.execute(
            "INSERT OR REPLACE INTO user_identities(provider,subject,user_id,email) VALUES(?,?,?,?)",
            vec![
                SqlValue::Text(provider.to_string()),
                SqlValue::Text(subject.to_string()),
                SqlValue::Text(user_id.to_string()),
                email.map(SqlValue::Text).unwrap_or(SqlValue::Null),
            ],
        )
        .await
    }

    async fn find_identities(&self, user_id: &UserId) -> AppResult<Vec<Identity>> {
        self.find_identities_impl(user_id).await
    }
}