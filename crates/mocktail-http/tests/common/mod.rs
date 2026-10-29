//! Shared in-memory fakes + harness for integration tests.
#![allow(dead_code)]

use async_trait::async_trait;
use mocktail_core::app::ctx::{AuthConfig, Ctx};
use mocktail_core::modules::auth::application::{MembershipRepository, UserRepository};
use mocktail_core::modules::auth::domain::{
    Identity, Membership, OAuthProvider, Role, User, new_session,
};
use mocktail_core::modules::collection::application::CollectionRepository;
use mocktail_core::modules::collection::domain::Collection;
use mocktail_core::modules::folder::application::FolderRepository;
use mocktail_core::modules::folder::domain::Folder;
use mocktail_core::modules::mock::application::MockRepository;
use mocktail_core::modules::mock::domain::Mock;
use mocktail_core::modules::request_log::application::{LogQuery, RequestLogRepository};
use mocktail_core::modules::request_log::domain::{RequestId, RequestLog};
use mocktail_core::modules::workspace::application::WorkspaceRepository;
use mocktail_core::modules::workspace::domain::Workspace;
use mocktail_core::shared::error::AppResult;
use mocktail_core::shared::http::{Headers, Method, Request, Response};
use mocktail_core::shared::ids::{
    CollectionId, FolderId, MembershipId, MockId, Slug, UserId, WorkspaceId,
};
use mocktail_core::shared::ports::{
    Clock, HttpClient, HttpResponse, IdGenerator, RateLimitDecision, RateLimiter, Signer, Timer,
    UnixMillis,
};
use mocktail_core::shared::rate_limit::FixedWindowState;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

pub const APEX: &str = "mocktail.test";

/// `RefCell`-shaped test state that is safe to share across axum tasks.
#[derive(Default)]
pub struct Shared<T>(Mutex<T>);

impl<T> Shared<T> {
    pub fn borrow(&self) -> MutexGuard<'_, T> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn borrow_mut(&self) -> MutexGuard<'_, T> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[derive(Default)]
pub struct FakeRepos {
    pub workspaces: Shared<Vec<Workspace>>,
    pub collections: Shared<Vec<Collection>>,
    pub folders: Shared<Vec<Folder>>,
    pub mocks: Shared<Vec<Mock>>,
    pub logs: Shared<Vec<RequestLog>>,
    pub users: Shared<Vec<User>>,
    pub memberships: Shared<Vec<Membership>>,
}

#[async_trait]
impl WorkspaceRepository for FakeRepos {
    async fn find_by_slug(&self, slug: &Slug) -> AppResult<Option<Workspace>> {
        Ok(self
            .workspaces
            .borrow()
            .iter()
            .find(|w| w.slug.as_str() == slug.as_str())
            .cloned())
    }
    async fn find_by_id(&self, id: &WorkspaceId) -> AppResult<Option<Workspace>> {
        Ok(self
            .workspaces
            .borrow()
            .iter()
            .find(|w| w.id.as_str() == id.as_str())
            .cloned())
    }
    async fn insert(&self, workspace: &Workspace) -> AppResult<()> {
        self.workspaces.borrow_mut().push(workspace.clone());
        Ok(())
    }
    async fn update(&self, workspace: &Workspace) -> AppResult<()> {
        let mut v = self.workspaces.borrow_mut();
        if let Some(slot) = v
            .iter_mut()
            .find(|w| w.id.as_str() == workspace.id.as_str())
        {
            *slot = workspace.clone();
        }
        Ok(())
    }
    async fn delete(&self, id: &WorkspaceId) -> AppResult<()> {
        let id = id.as_str();
        self.workspaces.borrow_mut().retain(|w| w.id.as_str() != id);
        self.collections
            .borrow_mut()
            .retain(|c| c.workspace_id.as_str() != id);
        self.folders
            .borrow_mut()
            .retain(|f| f.workspace_id.as_str() != id);
        self.mocks
            .borrow_mut()
            .retain(|m| m.workspace_id.as_str() != id);
        self.logs
            .borrow_mut()
            .retain(|l| l.workspace_id.as_str() != id);
        self.memberships
            .borrow_mut()
            .retain(|m| m.workspace_id.as_str() != id);
        Ok(())
    }
    async fn list(&self) -> AppResult<Vec<Workspace>> {
        Ok(self.workspaces.borrow().clone())
    }
}

#[async_trait]
impl CollectionRepository for FakeRepos {
    async fn find_by_slug(&self, ws: &WorkspaceId, slug: &Slug) -> AppResult<Option<Collection>> {
        Ok(self
            .collections
            .borrow()
            .iter()
            .find(|c| c.workspace_id.as_str() == ws.as_str() && c.slug.as_str() == slug.as_str())
            .cloned())
    }
    async fn find_by_id(&self, id: &CollectionId) -> AppResult<Option<Collection>> {
        Ok(self
            .collections
            .borrow()
            .iter()
            .find(|c| c.id.as_str() == id.as_str())
            .cloned())
    }
    async fn list(&self, ws: &WorkspaceId) -> AppResult<Vec<Collection>> {
        Ok(self
            .collections
            .borrow()
            .iter()
            .filter(|c| c.workspace_id.as_str() == ws.as_str())
            .cloned()
            .collect())
    }
    async fn insert(&self, collection: &Collection) -> AppResult<()> {
        self.collections.borrow_mut().push(collection.clone());
        Ok(())
    }
    async fn update(&self, collection: &Collection) -> AppResult<()> {
        let mut v = self.collections.borrow_mut();
        if let Some(slot) = v
            .iter_mut()
            .find(|c| c.id.as_str() == collection.id.as_str())
        {
            *slot = collection.clone();
        }
        Ok(())
    }
    async fn delete(&self, id: &CollectionId) -> AppResult<()> {
        let id = id.as_str();
        self.collections
            .borrow_mut()
            .retain(|c| c.id.as_str() != id);
        self.folders
            .borrow_mut()
            .retain(|f| f.collection_id.as_str() != id);
        self.mocks
            .borrow_mut()
            .retain(|m| m.collection_id.as_str() != id);
        Ok(())
    }
}

#[async_trait]
impl FolderRepository for FakeRepos {
    async fn list(&self, ws: &WorkspaceId, coll: &CollectionId) -> AppResult<Vec<Folder>> {
        Ok(self
            .folders
            .borrow()
            .iter()
            .filter(|f| {
                f.workspace_id.as_str() == ws.as_str() && f.collection_id.as_str() == coll.as_str()
            })
            .cloned()
            .collect())
    }
    async fn find_by_id(&self, id: &FolderId) -> AppResult<Option<Folder>> {
        Ok(self
            .folders
            .borrow()
            .iter()
            .find(|f| f.id.as_str() == id.as_str())
            .cloned())
    }
    async fn insert(&self, folder: &Folder) -> AppResult<()> {
        self.folders.borrow_mut().push(folder.clone());
        Ok(())
    }
    async fn delete(&self, id: &FolderId) -> AppResult<()> {
        self.folders
            .borrow_mut()
            .retain(|f| f.id.as_str() != id.as_str());
        Ok(())
    }
}

#[async_trait]
impl MockRepository for FakeRepos {
    async fn list_active(&self, ws: &WorkspaceId, coll: &CollectionId) -> AppResult<Vec<Mock>> {
        Ok(self
            .mocks
            .borrow()
            .iter()
            .filter(|m| {
                m.workspace_id.as_str() == ws.as_str() && m.collection_id.as_str() == coll.as_str()
            })
            .cloned()
            .collect())
    }
    async fn find_by_id(&self, id: &MockId) -> AppResult<Option<Mock>> {
        Ok(self
            .mocks
            .borrow()
            .iter()
            .find(|m| m.id.as_str() == id.as_str())
            .cloned())
    }
    async fn upsert(&self, mock: &Mock) -> AppResult<()> {
        let mut v = self.mocks.borrow_mut();
        if let Some(slot) = v.iter_mut().find(|m| m.id.as_str() == mock.id.as_str()) {
            *slot = mock.clone();
        } else {
            v.push(mock.clone());
        }
        Ok(())
    }
    async fn delete(&self, id: &MockId) -> AppResult<()> {
        self.mocks
            .borrow_mut()
            .retain(|m| m.id.as_str() != id.as_str());
        Ok(())
    }
}

#[async_trait]
impl RequestLogRepository for FakeRepos {
    async fn append(&self, log: &RequestLog) -> AppResult<()> {
        self.logs.borrow_mut().push(log.clone());
        Ok(())
    }
    async fn find_by_request_id(
        &self,
        ws: &WorkspaceId,
        request_id: &RequestId,
    ) -> AppResult<Option<RequestLog>> {
        Ok(self
            .logs
            .borrow()
            .iter()
            .find(|l| {
                l.workspace_id.as_str() == ws.as_str()
                    && l.request_id.as_str() == request_id.as_str()
            })
            .cloned())
    }
    async fn recent(
        &self,
        ws: &WorkspaceId,
        coll: Option<&CollectionId>,
        limit: usize,
    ) -> AppResult<Vec<RequestLog>> {
        Ok(self
            .logs
            .borrow()
            .iter()
            .rev()
            .filter(|l| l.workspace_id.as_str() == ws.as_str())
            .filter(|l| {
                coll.is_none_or(|c| {
                    l.collection_id.as_ref().map(|x| x.as_str()) == Some(c.as_str())
                })
            })
            .take(limit)
            .cloned()
            .collect())
    }
    async fn page(&self, ws: &WorkspaceId, q: &LogQuery<'_>) -> AppResult<Vec<RequestLog>> {
        Ok(self
            .logs
            .borrow()
            .iter()
            .rev()
            .filter(|l| l.workspace_id.as_str() == ws.as_str())
            .filter(|l| {
                q.coll.is_none_or(|c| {
                    l.collection_id.as_ref().map(|x| x.as_str()) == Some(c.as_str())
                })
            })
            .filter(|l| {
                q.path_contains
                    .is_none_or(|p| p.is_empty() || l.path.contains(p))
            })
            .filter(|l| {
                if let Some(s) = q.status {
                    l.status == s
                } else if let Some(c) = q.status_class {
                    l.status >= c && l.status < c + 100
                } else {
                    true
                }
            })
            .filter(|l| {
                q.cursor.as_ref().is_none_or(|cur| {
                    l.received_at < cur.received_at
                        || (l.received_at == cur.received_at && l.id.as_str() < cur.id.as_str())
                })
            })
            .take(q.limit)
            .cloned()
            .collect())
    }
}

#[async_trait]
impl UserRepository for FakeRepos {
    async fn find_by_identity(&self, _provider: &str, _subject: &str) -> AppResult<Option<User>> {
        Ok(self
            .users
            .borrow()
            .iter()
            .find(|_u| {
                // Simple fake: just return the first user for now or implement identity mapping
                false
            })
            .cloned())
    }
    async fn find_by_id(&self, id: &UserId) -> AppResult<Option<User>> {
        Ok(self
            .users
            .borrow()
            .iter()
            .find(|u| u.id.as_str() == id.as_str())
            .cloned())
    }
    async fn find_by_username(&self, username: &str) -> AppResult<Option<User>> {
        Ok(self
            .users
            .borrow()
            .iter()
            .find(|u| u.username == username)
            .cloned())
    }
    async fn insert(&self, user: &User) -> AppResult<()> {
        self.users.borrow_mut().push(user.clone());
        Ok(())
    }
    async fn update(&self, user: &User) -> AppResult<()> {
        let mut v = self.users.borrow_mut();
        if let Some(slot) = v.iter_mut().find(|u| u.id.as_str() == user.id.as_str()) {
            *slot = user.clone();
        }
        Ok(())
    }
    async fn add_identity(&self, _user_id: &UserId, _provider: &str, _subject: &str, _email: Option<String>) -> AppResult<()> {
        Ok(())
    }
    async fn find_identities(&self, _user_id: &UserId) -> AppResult<Vec<Identity>> {
        Ok(vec![])
    }
}

#[async_trait]
impl MembershipRepository for FakeRepos {
    async fn find(&self, user: &UserId, workspace: &WorkspaceId) -> AppResult<Option<Membership>> {
        Ok(self
            .memberships
            .borrow()
            .iter()
            .find(|m| {
                m.user_id.as_str() == user.as_str() && m.workspace_id.as_str() == workspace.as_str()
            })
            .cloned())
    }
    async fn list_for_user(&self, user: &UserId) -> AppResult<Vec<Membership>> {
        Ok(self
            .memberships
            .borrow()
            .iter()
            .filter(|m| m.user_id.as_str() == user.as_str())
            .cloned()
            .collect())
    }
    async fn list_for_workspace(&self, workspace: &WorkspaceId) -> AppResult<Vec<Membership>> {
        Ok(self
            .memberships
            .borrow()
            .iter()
            .filter(|m| m.workspace_id.as_str() == workspace.as_str())
            .cloned()
            .collect())
    }
    async fn insert(&self, membership: &Membership) -> AppResult<()> {
        self.memberships.borrow_mut().push(membership.clone());
        Ok(())
    }
    async fn update(&self, membership: &Membership) -> AppResult<()> {
        let mut v = self.memberships.borrow_mut();
        if let Some(slot) = v
            .iter_mut()
            .find(|m| m.id.as_str() == membership.id.as_str())
        {
            *slot = membership.clone();
        }
        Ok(())
    }
    async fn delete(&self, id: &MembershipId) -> AppResult<()> {
        self.memberships
            .borrow_mut()
            .retain(|m| m.id.as_str() != id.as_str());
        Ok(())
    }
}

pub struct FixedClock;
impl Clock for FixedClock {
    fn now_millis(&self) -> UnixMillis {
        1_700_000_000_000
    }
}

pub struct NoTimer;
#[async_trait]
impl Timer for NoTimer {
    async fn sleep_ms(&self, _ms: u32) {}
}

/// In-memory fixed-window limiter shared by a test context.
#[derive(Default)]
pub struct TestRateLimiter {
    state: Shared<FixedWindowState>,
}

impl RateLimiter for TestRateLimiter {
    fn check(&self, key: &str, limit: u32, window_secs: u32, now: UnixMillis) -> RateLimitDecision {
        self.state.borrow_mut().check(key, limit, window_secs, now)
    }
}

/// Deterministic test signer (NOT cryptographic): XOR mask.
pub struct FakeSigner;
impl Signer for FakeSigner {
    fn sign(&self, bytes: &[u8]) -> Vec<u8> {
        bytes.iter().map(|b| b ^ 0x5a).collect()
    }
}

/// HttpClient that fails every call (for tests that never hit the network).
pub struct NoHttp;
#[async_trait]
impl HttpClient for NoHttp {
    async fn post_form(
        &self,
        _url: &str,
        _form: &[(String, String)],
        _headers: &[(String, String)],
    ) -> Result<HttpResponse, String> {
        Err("no http".into())
    }
    async fn get(&self, _url: &str, _headers: &[(String, String)]) -> Result<HttpResponse, String> {
        Err("no http".into())
    }
    async fn post_json(
        &self,
        _url: &str,
        _body: &[u8],
        _headers: &[(String, String)],
    ) -> Result<HttpResponse, String> {
        Err("no http".into())
    }
}

/// HttpClient returning scripted JSON bodies by URL substring (OAuth tests).
pub struct ScriptedHttp {
    pub token_body: Vec<u8>,
    pub userinfo_body: Vec<u8>,
}
#[async_trait]
impl HttpClient for ScriptedHttp {
    async fn post_form(
        &self,
        _url: &str,
        _form: &[(String, String)],
        _headers: &[(String, String)],
    ) -> Result<HttpResponse, String> {
        Ok(HttpResponse {
            status: 200,
            body: self.token_body.clone(),
        })
    }
    async fn get(&self, _url: &str, _headers: &[(String, String)]) -> Result<HttpResponse, String> {
        Ok(HttpResponse {
            status: 200,
            body: self.userinfo_body.clone(),
        })
    }
    async fn post_json(
        &self,
        _url: &str,
        _body: &[u8],
        _headers: &[(String, String)],
    ) -> Result<HttpResponse, String> {
        Ok(HttpResponse {
            status: 200,
            body: self.token_body.clone(),
        })
    }
}

#[derive(Default)]
pub struct SeqIds {
    n: AtomicU64,
}
impl IdGenerator for SeqIds {
    fn new_object_id(&self) -> String {
        let n = self.n.fetch_add(1, Ordering::Relaxed) + 1;
        uuid::Uuid::new_v4().to_string()
    }
    fn new_request_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

/// A fully wired test context plus a pre-seeded, signed-in user.
pub struct Harness {
    pub ctx: Arc<Ctx>,
    pub repos: Arc<FakeRepos>,
    /// Session cookie value for the seeded user.
    pub session: String,
    /// CSRF token tied to that session.
    pub csrf: String,
    pub user_id: String,
}

/// Build a context with custom http + providers (used by the OAuth test).
pub fn make_ctx(
    repos: Arc<FakeRepos>,
    http: Arc<dyn HttpClient>,
    providers: HashMap<String, OAuthProvider>,
) -> Arc<Ctx> {
    Arc::new(Ctx {
        apex_host: APEX.to_string(),
        workspaces: repos.clone(),
        collections: repos.clone(),
        folders: repos.clone(),
        mocks: repos.clone(),
        request_logs: repos.clone(),
        users: repos.clone(),
        memberships: repos.clone(),
        rate_limiter: Arc::new(TestRateLimiter::default()),
        clock: Arc::new(FixedClock),
        ids: Arc::new(SeqIds::default()),
        http,
        signer: Arc::new(FakeSigner),
        timer: Arc::new(NoTimer),
        auth: Arc::new(AuthConfig {
            providers,
            public_base_url: format!("https://{APEX}"),
            session_ttl_ms: 3_600_000,
            secure_cookies: false,
        }),
    })
}

/// Default harness: no providers, no network, one signed-in user.
pub fn harness() -> Harness {
    let repos = Arc::new(FakeRepos::default());
    let ctx = make_ctx(repos.clone(), Arc::new(NoHttp), HashMap::new());

    // Seed a user and mint a session for it.
    let user = User {
        id: UserId::new("seeduser0000000000000001"),
        username: "seeduser".into(),
        display_name: "Dev".into(),
        created_at: 0,
    };
    repos.users.borrow_mut().push(user.clone());
    let (session, claims) = new_session(
        &FakeSigner,
        &SeqIds::default(),
        &user.id,
        FixedClock.now_millis(),
        3_600_000,
    );
    Harness {
        ctx,
        repos,
        session,
        csrf: claims.csrf,
        user_id: user.id.as_str().to_string(),
    }
}

/// Grant the seeded user a role in a workspace (helper for RBAC tests).
pub fn grant(h: &Harness, workspace_id: &str, role: Role) {
    h.repos.memberships.borrow_mut().push(Membership {
        id: MembershipId::new(format!("m-{workspace_id}")),
        user_id: UserId::new(h.user_id.clone()),
        workspace_id: WorkspaceId::new(workspace_id),
        role,
    });
}

fn base_req(h: &Harness, method: Method, path: &str, body: Vec<u8>) -> Request {
    let mut headers = Headers::new();
    headers.insert("cookie", format!("mocktail_session={}", h.session));
    headers.insert("x-csrf-token", h.csrf.clone());
    Request {
        method,
        host: APEX.to_string(),
        path: path.to_string(),
        query: vec![],
        headers,
        body,
        client_ip: Some("203.0.113.7".into()),
    }
}

/// Authenticated admin request with a JSON body.
pub fn areq(h: &Harness, method: Method, path: &str, body: serde_json::Value) -> Request {
    let body = if body.is_null() {
        vec![]
    } else {
        serde_json::to_vec(&body).unwrap()
    };
    base_req(h, method, path, body)
}

/// Authenticated admin request with a raw body (e.g. an uploaded spec).
pub fn araw(h: &Harness, method: Method, path: &str, body: Vec<u8>) -> Request {
    base_req(h, method, path, body)
}

/// An unauthenticated admin request (for testing the auth gate).
pub fn anon(method: Method, path: &str, body: serde_json::Value) -> Request {
    Request {
        method,
        host: APEX.to_string(),
        path: path.to_string(),
        query: vec![],
        headers: Headers::new(),
        body: if body.is_null() {
            vec![]
        } else {
            serde_json::to_vec(&body).unwrap()
        },
        client_ip: Some("203.0.113.7".into()),
    }
}

pub fn mock_req(host_label: &str, path: &str) -> Request {
    Request {
        method: Method::Get,
        host: format!("{host_label}.{APEX}"),
        path: path.to_string(),
        query: vec![],
        headers: Headers::new(),
        body: vec![],
        client_ip: Some("203.0.113.7".into()),
    }
}

pub fn body_json(resp: &Response) -> serde_json::Value {
    serde_json::from_slice(&resp.body).unwrap_or(serde_json::Value::Null)
}

/// Parse an admin/dashboard response and return its `data` payload.
pub fn admin_json(resp: &Response) -> serde_json::Value {
    let body = body_json(resp);
    body.get("data").cloned().unwrap_or(body)
}

/// Minimal single-threaded executor so tests need no tokio dependency. All
/// fakes are synchronous, so each future is Ready on the first poll.
pub fn block_on<F: std::future::Future>(mut fut: F) -> F::Output {
    use std::pin::Pin;
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    fn noop_raw_waker() -> RawWaker {
        fn no_op(_: *const ()) {}
        fn clone(_: *const ()) -> RawWaker {
            noop_raw_waker()
        }
        let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
        RawWaker::new(std::ptr::null(), vtable)
    }

    let waker = unsafe { Waker::from_raw(noop_raw_waker()) };
    let mut cx = Context::from_waker(&waker);
    let mut fut = unsafe { Pin::new_unchecked(&mut fut) };
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return v,
            Poll::Pending => continue,
        }
    }
}
