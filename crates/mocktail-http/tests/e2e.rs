//! End-to-end flow through `handle` with in-memory fakes.
//! Drives the authenticated admin API to create a workspace + collection +
//! mock, then serves the mock on the public runtime surface.

mod common;

use common::{APEX, admin_json, areq, block_on, body_json, harness, mock_req};
use mocktail_core::shared::http::{Method, Request};
use mocktail_http::handle;

#[test]
fn full_flow_import_then_serve() {
    block_on(async {
        let h = harness();
        let ctx = &h.ctx;

        // 1. Create workspace (the signed-in user becomes its owner).
        let resp = handle(
            areq(
                &h,
                Method::Post,
                "/__admin/workspaces",
                serde_json::json!({"name": "Acme"}),
            ),
            ctx,
        )
        .await;
        assert_eq!(
            resp.status,
            201,
            "{:?}",
            String::from_utf8_lossy(&resp.body)
        );
        let ws = admin_json(&resp);
        assert_eq!(ws["slug"], "acme");
        let ws_id = ws["id"].as_str().unwrap().to_string();

        // 2. Create collection with a base path.
        let resp = handle(
            areq(
                &h,
                Method::Post,
                &format!("/__admin/workspaces/{ws_id}/collections"),
                serde_json::json!({"name": "Todo", "basePath": "/api/v1"}),
            ),
            ctx,
        )
        .await;
        assert_eq!(
            resp.status,
            201,
            "{:?}",
            String::from_utf8_lossy(&resp.body)
        );
        let coll = admin_json(&resp);
        assert_eq!(coll["slug"], "todo");
        assert_eq!(coll["mountPath"], "/todo/api/v1");
        let coll_id = coll["id"].as_str().unwrap().to_string();

        // 3. Create a mock with a templated body.
        let resp = handle(
            areq(
                &h,
                Method::Post,
                &format!("/__admin/collections/{coll_id}/mocks"),
                serde_json::json!({
                    "method": "GET",
                    "pathTemplate": "/todos/{id}",
                    "response": {"status": 200, "body": "{\"id\":\"{{id}}\"}"}
                }),
            ),
            ctx,
        )
        .await;
        assert_eq!(
            resp.status,
            201,
            "{:?}",
            String::from_utf8_lossy(&resp.body)
        );

        // 4. Serve the mock from the workspace subdomain at the full mount URL
        //    (public runtime surface, no auth).
        let mut serve = mock_req("acme", "/todo/api/v1/todos/42");
        serve.headers.insert("Origin", "https://app.example.com");
        let resp = handle(serve, ctx).await;
        assert_eq!(resp.status, 200);
        assert_eq!(String::from_utf8_lossy(&resp.body), "{\"id\":\"42\"}");
        assert_eq!(resp.headers.get("X-Request-Id"), Some("generated-req-id"));
        assert_eq!(resp.headers.get("Access-Control-Allow-Origin"), Some("*"));

        // 5. A path with no matching mock yields a diagnostic 404 with near-misses.
        let resp = handle(mock_req("acme", "/todo/api/v1/nope"), ctx).await;
        assert_eq!(resp.status, 404);
        let b = body_json(&resp);
        assert_eq!(b["error"], "no matching mock");
        assert_eq!(b["nearMisses"][0], "GET /todos/{id}");

        // 6. CORS preflight answered automatically.
        let mut pre = Request {
            method: Method::Options,
            host: format!("acme.{APEX}"),
            path: "/todo/api/v1/todos/42".to_string(),
            query: vec![],
            headers: Default::default(),
            body: vec![],
            client_ip: Some("203.0.113.7".into()),
        };
        pre.headers.insert("Origin", "https://app.example.com");
        let resp = handle(pre, ctx).await;
        assert_eq!(resp.status, 204);
        assert!(resp.headers.get("Access-Control-Allow-Methods").is_some());

        // 7. Request logs recorded (serve + 404 + preflight = 3).
        assert_eq!(h.repos.logs.borrow().len(), 3);
    });
}
