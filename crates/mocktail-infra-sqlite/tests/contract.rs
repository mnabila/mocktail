//! Repository contract test for `SqliteRepositories`, run against
//! a real in-memory SQLite via sqlx — the same SQL D1 executes.

use async_trait::async_trait;
use mocktail_core::modules::auth::application::{MembershipRepository, UserRepository};
use mocktail_core::modules::auth::domain::{Identity, Membership, Role, User};
use mocktail_core::modules::collection::application::CollectionRepository;
use mocktail_core::modules::collection::domain::{Collection, CorsPolicy, Source};
use mocktail_core::modules::folder::application::FolderRepository;
use mocktail_core::modules::folder::domain::Folder;
use mocktail_core::modules::mock::application::MockRepository;
use mocktail_core::modules::mock::domain::{Mock, Priority, RateLimit, ResponseSpec};
use mocktail_core::modules::request_log::application::RequestLogRepository;
use mocktail_core::modules::request_log::domain::{RequestId, RequestLog, ResponseLog};
use mocktail_core::modules::workspace::application::WorkspaceRepository;
use mocktail_core::modules::workspace::domain::Workspace;
use mocktail_core::shared::http::Method;
use mocktail_core::shared::ids::{
    CollectionId, FolderId, Id, MembershipId, MockId, Slug, UserId, WorkspaceId,
};
use mocktail_infra_sqlite::{SqlExecutor, SqlValue, SqliteRepositories};
use serde_json::{Map, Value};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Column, Row, SqlitePool};
use std::sync::Arc;

/// Decode one column into JSON without knowing its declared type up front.
/// SQLite is dynamically typed, so we probe the common storage classes in order
/// (integer, real, text) and fall back to Null, matching the native executor.
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

/// sqlx-backed executor matching the `SqlExecutor` contract D1 provides.
struct SqlxExec {
    pool: SqlitePool,
}

#[async_trait]
impl SqlExecutor for SqlxExec {
    async fn query(&self, sql: &str, params: Vec<SqlValue>) -> Result<Vec<Value>, String> {
        let mut q = sqlx::query(sql);
        for p in params {
            q = match p {
                SqlValue::Text(s) => q.bind(s),
                SqlValue::I64(n) => q.bind(n),
                SqlValue::Bool(b) => q.bind(b),
                SqlValue::Real(f) => q.bind(f),
                SqlValue::Null => q.bind(Option::<String>::None),
            };
        }
        let rows = q.fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in &rows {
            let mut obj = Map::new();
            for col in row.columns() {
                obj.insert(col.name().to_string(), decode_col(row, col.ordinal()));
            }
            out.push(Value::Object(obj));
        }
        Ok(out)
    }

    async fn execute(&self, sql: &str, params: Vec<SqlValue>) -> Result<(), String> {
        let mut q = sqlx::query(sql);
        for p in params {
            q = match p {
                SqlValue::Text(s) => q.bind(s),
                SqlValue::I64(n) => q.bind(n),
                SqlValue::Bool(b) => q.bind(b),
                SqlValue::Real(f) => q.bind(f),
                SqlValue::Null => q.bind(Option::<String>::None),
            };
        }
        q.execute(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

async fn repos() -> SqliteRepositories {
    // max_connections(1) so the in-memory database is shared across queries.
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let exec = Arc::new(SqlxExec { pool });
    SqliteRepositories::new(exec)
}

#[tokio::test(flavor = "current_thread")]
async fn ensure_schema_is_idempotent() {
    let repo = repos().await;
    repo.ensure_schema().await.unwrap();
    // Re-running on an already-migrated database must be a no-op.
    repo.ensure_schema().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn sqlite_round_trips_all_ports() {
    let repo = repos().await;
    repo.ensure_schema().await.unwrap();

    // workspace
    WorkspaceRepository::insert(
        &repo,
        &Workspace {
            id: WorkspaceId::new("w1"),
            name: "Acme".into(),
            slug: Slug::parse("acme").unwrap(),
            created_at: 1,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        WorkspaceRepository::find_by_slug(&repo, &Slug::parse("acme").unwrap())
            .await
            .unwrap()
            .unwrap()
            .name,
        "Acme"
    );
    assert_eq!(WorkspaceRepository::list(&repo).await.unwrap().len(), 1);

    // collection
    CollectionRepository::insert(
        &repo,
        &Collection {
            id: CollectionId::new("c1"),
            workspace_id: WorkspaceId::new("w1"),
            name: "todo".into(),
            slug: Slug::parse("todo").unwrap(),
            description: "".into(),
            source: Source::Manual,
            base_path: "/api/v1".into(),
            cors_policy: CorsPolicy::default(),
            version: 1,
            created_at: 1,
        },
    )
    .await
    .unwrap();
    assert!(
        CollectionRepository::find_by_slug(
            &repo,
            &WorkspaceId::new("w1"),
            &Slug::parse("todo").unwrap()
        )
        .await
        .unwrap()
        .is_some()
    );

    // mock upsert replaces, not duplicates
    let mut mock = Mock {
        id: MockId::new("m1"),
        workspace_id: WorkspaceId::new("w1"),
        collection_id: CollectionId::new("c1"),
        folder_id: None,
        method: Method::Get,
        path_template: "/todos/{id}".into(),
        matchers: vec![],
        response: ResponseSpec {
            status: 200,
            headers: vec![("content-type".into(), "application/json".into())],
            body: b"{}".to_vec(),
        },
        priority: Priority(0),
        delay_ms: 0,
        fault: None,
        rate_limit: Some(RateLimit {
            limit: 10,
            window_secs: 60,
            status: 429,
        }),
        created_at: 1,
        updated_at: 1,
    };
    MockRepository::upsert(&repo, &mock).await.unwrap();
    mock.response.status = 201;
    MockRepository::upsert(&repo, &mock).await.unwrap();
    let active =
        MockRepository::list_active(&repo, &WorkspaceId::new("w1"), &CollectionId::new("c1"))
            .await
            .unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].response.status, 201);
    assert_eq!(
        active[0].rate_limit,
        Some(RateLimit {
            limit: 10,
            window_secs: 60,
            status: 429,
        })
    );

    // folder round-trip + mock association
    FolderRepository::insert(
        &repo,
        &Folder {
            id: FolderId::new("f1"),
            workspace_id: WorkspaceId::new("w1"),
            collection_id: CollectionId::new("c1"),
            name: "Users".into(),
            parent_id: None,
            created_at: 1,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        FolderRepository::list(&repo, &WorkspaceId::new("w1"), &CollectionId::new("c1"))
            .await
            .unwrap()
            .len(),
        1
    );
    mock.folder_id = Some(FolderId::new("f1"));
    MockRepository::upsert(&repo, &mock).await.unwrap();
    assert_eq!(
        MockRepository::find_by_id(&repo, &MockId::new("m1"))
            .await
            .unwrap()
            .unwrap()
            .folder_id,
        Some(FolderId::new("f1"))
    );

    // collection delete cascades to mocks and folders
    CollectionRepository::delete(&repo, &CollectionId::new("c1"))
        .await
        .unwrap();
    assert!(
        MockRepository::list_active(&repo, &WorkspaceId::new("w1"), &CollectionId::new("c1"))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        FolderRepository::list(&repo, &WorkspaceId::new("w1"), &CollectionId::new("c1"))
            .await
            .unwrap()
            .is_empty()
    );

    // user + identity index, and connect adds a second identity
    let mut user = User {
        id: UserId::new("u1"),
        username: "devuser".into(),
        display_name: "A".into(),
        created_at: 1,
    };
    UserRepository::insert(&repo, &user).await.unwrap();
    assert_eq!(
        UserRepository::find_by_identity(&repo, "github", "42")
            .await
            .unwrap()
            .unwrap()
            .id
            .as_str(),
        "u1"
    );
    assert_eq!(
        UserRepository::find_by_username(&repo, "devuser")
            .await
            .unwrap()
            .unwrap()
            .id
            .as_str(),
        "u1"
    );
    assert!(
        UserRepository::find_by_username(&repo, "nobody")
            .await
            .unwrap()
            .is_none()
    );
    // For the "connect" part of the test, we now use add_identity
    UserRepository::add_identity(&repo, &UserId::new("u1"), "google", "g", None).await.unwrap();
    assert!(
        UserRepository::find_by_identity(&repo, "google", "g")
            .await
            .unwrap()
            .is_some()
    );

    // membership
    MembershipRepository::insert(
        &repo,
        &Membership {
            id: MembershipId::new("mem1"),
            user_id: UserId::new("u1"),
            workspace_id: WorkspaceId::new("w1"),
            role: Role::Viewer,
        },
    )
    .await
    .unwrap();
    let mut m = MembershipRepository::find(&repo, &UserId::new("u1"), &WorkspaceId::new("w1"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(m.role, Role::Viewer);
    m.role = Role::Owner;
    MembershipRepository::update(&repo, &m).await.unwrap();
    assert_eq!(
        MembershipRepository::list_for_workspace(&repo, &WorkspaceId::new("w1"))
            .await
            .unwrap()[0]
            .role,
        Role::Owner
    );

    // request logs sorted desc + limited
    for (i, ts) in [(1, 10), (2, 30), (3, 20)] {
        RequestLogRepository::append(
            &repo,
            &RequestLog {
                id: Id::new(format!("l{i}")),
                request_id: RequestId(format!("r{i}")),
                workspace_id: WorkspaceId::new("w1"),
                collection_id: None,
                received_at: ts,
                method: "GET".into(),
                path: "/x".into(),
                request_headers: vec![("accept".into(), "application/json".into())],
                request_body: "{\"a\":1}".into(),
                matched_mock_id: None,
                match_reason: None,
                status: 200,
                ip: None,
                country: None,
                lat: None,
                lng: None,
                response: ResponseLog {
                    status: 200,
                    headers: vec![("content-type".into(), "application/json".into())],
                    body: "{}".into(),
                },
            },
        )
        .await
        .unwrap();
    }
    let recent = RequestLogRepository::recent(&repo, &WorkspaceId::new("w1"), None, 2)
        .await
        .unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].received_at, 30);
    assert_eq!(recent[1].received_at, 20);
    assert_eq!(
        RequestLogRepository::find_by_request_id(
            &repo,
            &WorkspaceId::new("w1"),
            &RequestId("r2".into())
        )
        .await
        .unwrap()
        .unwrap()
        .received_at,
        30
    );

    // workspace update + delete cascades to all child rows
    let mut w = WorkspaceRepository::find_by_id(&repo, &WorkspaceId::new("w1"))
        .await
        .unwrap()
        .unwrap();
    w.name = "Acme 2".into();
    WorkspaceRepository::update(&repo, &w).await.unwrap();
    assert_eq!(
        WorkspaceRepository::find_by_id(&repo, &WorkspaceId::new("w1"))
            .await
            .unwrap()
            .unwrap()
            .name,
        "Acme 2"
    );
    WorkspaceRepository::delete(&repo, &WorkspaceId::new("w1"))
        .await
        .unwrap();
    assert!(WorkspaceRepository::list(&repo).await.unwrap().is_empty());
    assert!(
        CollectionRepository::list(&repo, &WorkspaceId::new("w1"))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        RequestLogRepository::recent(&repo, &WorkspaceId::new("w1"), None, 10)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        MembershipRepository::list_for_workspace(&repo, &WorkspaceId::new("w1"))
            .await
            .unwrap()
            .is_empty()
    );
    // Users are deployment-level, not workspace children.
    assert!(
        UserRepository::find_by_id(&repo, &UserId::new("u1"))
            .await
            .unwrap()
            .is_some()
    );
}
