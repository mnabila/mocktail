//! Auth & RBAC end-to-end: the unauthenticated gate, a full OAuth login via a
//! scripted provider, role enforcement, and explicit account linking.

mod common;

use common::{
    APEX, FakeRepos, ScriptedHttp, admin_json, anon, areq, block_on, grant, harness, make_ctx,
};
use mocktail_core::modules::auth::domain::{OAuthProvider, new_session};
use mocktail_core::shared::http::{Headers, Method, Request};
use mocktail_http::handle;
use std::collections::HashMap;
use std::sync::Arc;

#[test]
fn admin_api_requires_session() {
    block_on(async {
        let h = harness();
        // No cookie => 401.
        let resp = handle(
            anon(Method::Get, "/__admin/workspaces", serde_json::Value::Null),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 401);

        // Health is open.
        let resp = handle(
            anon(Method::Get, "/__admin/health", serde_json::Value::Null),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 200);
    });
}

#[test]
fn mutation_requires_csrf() {
    block_on(async {
        let h = harness();
        // Authenticated but missing the CSRF header => 403.
        let mut req = Request {
            method: Method::Post,
            host: APEX.to_string(),
            path: "/__admin/workspaces".to_string(),
            query: vec![],
            headers: Headers::new(),
            body: serde_json::to_vec(&serde_json::json!({"name": "X"})).unwrap(),
            client_ip: Some("203.0.113.7".into()),
        };
        req.headers
            .insert("cookie", format!("mocktail_session={}", h.session));
        let resp = handle(req, &h.ctx).await;
        assert_eq!(resp.status, 403);
    });
}

#[test]
fn rbac_viewer_cannot_mutate() {
    block_on(async {
        let h = harness();
        // Create a workspace (actor becomes owner).
        let ws = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Post,
                    "/__admin/workspaces",
                    serde_json::json!({"name": "Acme"}),
                ),
                &h.ctx,
            )
            .await,
        );
        let ws_id = ws["id"].as_str().unwrap().to_string();

        // A second user who is only a viewer cannot create a collection.
        let viewer = harness_sharing(&h);
        grant(
            &viewer,
            &ws_id,
            mocktail_core::modules::auth::domain::Role::Viewer,
        );
        let resp = handle(
            areq(
                &viewer,
                Method::Post,
                &format!("/__admin/workspaces/{ws_id}/collections"),
                serde_json::json!({"name": "Todo"}),
            ),
            &viewer.ctx,
        )
        .await;
        assert_eq!(
            resp.status,
            403,
            "{:?}",
            String::from_utf8_lossy(&resp.body)
        );

        // The owner can.
        let resp = handle(
            areq(
                &h,
                Method::Post,
                &format!("/__admin/workspaces/{ws_id}/collections"),
                serde_json::json!({"name": "Todo"}),
            ),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 201);
    });
}

/// Build a second signed-in user that shares the same repos as `h` (so both
/// act on the same workspace), returning a harness-like bundle for `areq`.
fn harness_sharing(h: &common::Harness) -> common::Harness {
    use mocktail_core::modules::auth::domain::{Identity, User};
    use mocktail_core::shared::ids::UserId;
    use mocktail_core::shared::ports::Clock;

    let user = User {
        id: UserId::new("seconduser00000000000001"),
        username: "vieweruser".into(),
        display_name: "Viewer".into(),
        created_at: 0,
    };
    h.repos.users.borrow_mut().push(user.clone());
    let (session, claims) = new_session(
        &common::FakeSigner,
        &common::SeqIds::default(),
        &user.id,
        common::FixedClock.now_millis(),
        3_600_000,
    );
    common::Harness {
        ctx: h.ctx.clone(),
        repos: h.repos.clone(),
        session,
        csrf: claims.csrf,
        user_id: user.id.as_str().to_string(),
    }
}

#[test]
fn oauth_login_creates_user_and_session() {
    block_on(async {
        let repos = Arc::new(FakeRepos::default());
        let http = Arc::new(ScriptedHttp {
            token_body: br#"{"access_token":"at","token_type":"bearer"}"#.to_vec(),
            userinfo_body: br#"{"sub":"abc123","email":"new@example.com"}"#.to_vec(),
        });
        let mut providers = HashMap::new();
        providers.insert("google".to_string(), google_provider());
        let ctx = make_ctx(repos.clone(), http, providers);

        // 1. Begin login: 302 to provider with an oauth_state cookie.
        let resp = handle(
            Request {
                method: Method::Get,
                host: APEX.to_string(),
                path: "/auth/google/login".to_string(),
                query: vec![],
                headers: Headers::new(),
                body: vec![],
                client_ip: Some("203.0.113.7".into()),
            },
            &ctx,
        )
        .await;
        assert_eq!(resp.status, 302);
        let location = resp.headers.get("location").unwrap().to_string();
        assert!(location.contains("accounts.google.com"));
        let set_cookie = resp.headers.get("set-cookie").unwrap();
        let state = set_cookie
            .strip_prefix("mocktail_oauth=")
            .and_then(|s| s.split(';').next())
            .unwrap()
            .to_string();

        // 2. Callback with the matching state + cookie => session set, user made.
        let mut cb = Request {
            method: Method::Get,
            host: APEX.to_string(),
            path: "/auth/google/callback".to_string(),
            query: vec![
                ("code".to_string(), "authcode".to_string()),
                ("state".to_string(), state.clone()),
            ],
            headers: Headers::new(),
            body: vec![],
            client_ip: Some("203.0.113.7".into()),
        };
        cb.headers
            .insert("cookie", format!("mocktail_oauth={state}"));
        let resp = handle(cb, &ctx).await;
        assert_eq!(
            resp.status,
            302,
            "{:?}",
            String::from_utf8_lossy(&resp.body)
        );
        assert_eq!(resp.headers.get("location"), Some("/"));
        assert!(resp.headers.iter().any(
            |(k, v)| k.eq_ignore_ascii_case("set-cookie") && v.starts_with("mocktail_session=")
        ));
        // The user was created by (provider, subject).
        assert_eq!(repos.users.borrow().len(), 1);
        assert!(repos.users.borrow()[0].has_identity("google", "abc123"));
    });
}

#[test]
fn callback_rejects_state_mismatch() {
    block_on(async {
        let repos = Arc::new(FakeRepos::default());
        let http = Arc::new(ScriptedHttp {
            token_body: b"{}".to_vec(),
            userinfo_body: b"{}".to_vec(),
        });
        let mut providers = HashMap::new();
        providers.insert("google".to_string(), google_provider());
        let ctx = make_ctx(repos, http, providers);

        // Cookie state and query state differ => 401 (CSRF guard).
        let mut cb = Request {
            method: Method::Get,
            host: APEX.to_string(),
            path: "/auth/google/callback".to_string(),
            query: vec![
                ("code".to_string(), "c".to_string()),
                ("state".to_string(), "aaa".to_string()),
            ],
            headers: Headers::new(),
            body: vec![],
            client_ip: Some("203.0.113.7".into()),
        };
        cb.headers.insert("cookie", "mocktail_oauth=bbb");
        let resp = handle(cb, &ctx).await;
        assert_eq!(resp.status, 401);
    });
}

#[test]
fn connect_provider_links_and_rejects_when_taken() {
    use mocktail_core::modules::auth::domain::{Identity, User};
    use mocktail_core::shared::ids::UserId;
    use mocktail_core::shared::ports::Clock;

    block_on(async {
        let repos = Arc::new(FakeRepos::default());
        // Signed-in user with only a github identity.
        let user = User {
            id: UserId::new("linkuser0000000000000001"),
            providers: vec![Identity {
                provider: "github".into(),
                subject: "1".into(),
                email: Some("me@example.com".into()),
            }],
            display_name: "Me".into(),
            created_at: 0,
        };
        repos.users.borrow_mut().push(user.clone());

        let http = Arc::new(ScriptedHttp {
            token_body: br#"{"access_token":"at"}"#.to_vec(),
            userinfo_body: br#"{"sub":"g-new","email":"me@example.com"}"#.to_vec(),
        });
        let mut providers = HashMap::new();
        providers.insert("google".to_string(), google_provider());
        let ctx = make_ctx(repos.clone(), http, providers);

        let (session, _claims) = new_session(
            &common::FakeSigner,
            &common::SeqIds::default(),
            &user.id,
            common::FixedClock.now_millis(),
            3_600_000,
        );

        // Begin connect (needs the session) -> 302 + state cookie.
        let mut begin = Request {
            method: Method::Get,
            host: APEX.to_string(),
            path: "/auth/google/connect".to_string(),
            query: vec![],
            headers: Headers::new(),
            body: vec![],
            client_ip: Some("203.0.113.7".into()),
        };
        begin
            .headers
            .insert("cookie", format!("mocktail_session={session}"));
        let resp = handle(begin, &ctx).await;
        assert_eq!(resp.status, 302);
        let state = resp
            .headers
            .get("set-cookie")
            .unwrap()
            .strip_prefix("mocktail_oauth=")
            .and_then(|s| s.split(';').next())
            .unwrap()
            .to_string();

        // Callback links the google identity to the existing user.
        let mut cb = Request {
            method: Method::Get,
            host: APEX.to_string(),
            path: "/auth/google/callback".to_string(),
            query: vec![
                ("code".to_string(), "c".to_string()),
                ("state".to_string(), state.clone()),
            ],
            headers: Headers::new(),
            body: vec![],
            client_ip: Some("203.0.113.7".into()),
        };
        cb.headers.insert(
            "cookie",
            format!("mocktail_session={session}; mocktail_oauth={state}"),
        );
        let resp = handle(cb, &ctx).await;
        assert_eq!(
            resp.status,
            302,
            "{:?}",
            String::from_utf8_lossy(&resp.body)
        );
        assert_eq!(repos.users.borrow().len(), 1);
        assert!(repos.users.borrow()[0].has_identity("google", "g-new"));

        // A different user already owns a google identity "g-taken".
        repos.users.borrow_mut().push(User {
            id: UserId::new("otheruser0000000000000001"),
            providers: vec![Identity {
                provider: "google".into(),
                subject: "g-taken".into(),
                email: None,
            }],
            display_name: "Other".into(),
            created_at: 0,
        });
        let http2 = Arc::new(ScriptedHttp {
            token_body: br#"{"access_token":"at"}"#.to_vec(),
            userinfo_body: br#"{"sub":"g-taken","email":"x@example.com"}"#.to_vec(),
        });
        let mut providers2 = HashMap::new();
        providers2.insert("google".to_string(), google_provider());
        let ctx2 = make_ctx(repos.clone(), http2, providers2);

        let mut begin2 = Request {
            method: Method::Get,
            host: APEX.to_string(),
            path: "/auth/google/connect".to_string(),
            query: vec![],
            headers: Headers::new(),
            body: vec![],
            client_ip: Some("203.0.113.7".into()),
        };
        begin2
            .headers
            .insert("cookie", format!("mocktail_session={session}"));
        let resp = handle(begin2, &ctx2).await;
        let state2 = resp
            .headers
            .get("set-cookie")
            .unwrap()
            .strip_prefix("mocktail_oauth=")
            .and_then(|s| s.split(';').next())
            .unwrap()
            .to_string();
        let mut cb2 = Request {
            method: Method::Get,
            host: APEX.to_string(),
            path: "/auth/google/callback".to_string(),
            query: vec![
                ("code".to_string(), "c".to_string()),
                ("state".to_string(), state2.clone()),
            ],
            headers: Headers::new(),
            body: vec![],
            client_ip: Some("203.0.113.7".into()),
        };
        cb2.headers.insert(
            "cookie",
            format!("mocktail_session={session}; mocktail_oauth={state2}"),
        );
        let resp = handle(cb2, &ctx2).await;
        // Already linked elsewhere => 409 conflict.
        assert_eq!(
            resp.status,
            409,
            "{:?}",
            String::from_utf8_lossy(&resp.body)
        );
    });
}

fn google_provider() -> OAuthProvider {
    OAuthProvider {
        name: "google".into(),
        client_id: "cid".into(),
        client_secret: "sec".into(),
        auth_url: "https://accounts.google.com/o/oauth2/v2/auth".into(),
        token_url: "https://oauth2.googleapis.com/token".into(),
        userinfo_url: "https://openidconnect.googleapis.com/v1/userinfo".into(),
        scopes: vec!["openid".into(), "email".into()],
        subject_field: "sub".into(),
        email_field: "email".into(),
    }
}
