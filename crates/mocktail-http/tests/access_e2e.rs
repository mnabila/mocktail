//! Per-collection IP allowlist: the mock runtime rejects client IPs not on a
//! collection's allowlist, and serves everyone when the list is empty.

mod common;

use common::{admin_json, areq, block_on, harness, mock_req};
use mocktail_core::shared::http::{Method, Request};
use mocktail_http::handle;

/// A runtime request carrying its client IP via `X-Forwarded-For` - the admin
/// `handle()` path rebuilds the request through axum, which derives the client
/// IP from headers (the `Request.client_ip` field is not carried across).
fn ping_from(host: &str, ip: &str) -> Request {
    let mut req = mock_req(host, "/todo/ping");
    req.headers.insert("x-forwarded-for", ip);
    req
}

async fn make_collection(
    h: &common::Harness,
    name: &str,
    ip_allowlist: serde_json::Value,
) -> String {
    let ws = admin_json(
        &handle(
            areq(
                h,
                Method::Post,
                "/__admin/workspaces",
                serde_json::json!({ "name": name }),
            ),
            &h.ctx,
        )
        .await,
    );
    let ws_id = ws["id"].as_str().unwrap().to_string();
    let coll = admin_json(
        &handle(
            areq(
                h,
                Method::Post,
                &format!("/__admin/workspaces/{ws_id}/collections"),
                serde_json::json!({ "name": "Todo", "basePath": "", "ipAllowlist": ip_allowlist }),
            ),
            &h.ctx,
        )
        .await,
    );
    let coll_id = coll["id"].as_str().unwrap().to_string();
    handle(
        areq(
            h,
            Method::Post,
            &format!("/__admin/collections/{coll_id}/mocks"),
            serde_json::json!({
                "method": "GET",
                "pathTemplate": "/ping",
                "response": { "status": 200, "body": "pong" }
            }),
        ),
        &h.ctx,
    )
    .await;
    coll_id
}

#[test]
fn allowlist_permits_listed_blocks_others() {
    block_on(async {
        let h = harness();
        make_collection(&h, "Acme", serde_json::json!(["203.0.113.7", "10.0.0.0/8"])).await;

        // Listed IP is served.
        let resp = handle(ping_from("acme", "203.0.113.7"), &h.ctx).await;
        assert_eq!(resp.status, 200);
        assert_eq!(String::from_utf8_lossy(&resp.body), "pong");

        // A CIDR-covered IP is served too.
        assert_eq!(
            handle(ping_from("acme", "10.9.9.9"), &h.ctx).await.status,
            200
        );

        // An off-list IP gets 403, not the mock.
        let resp = handle(ping_from("acme", "198.51.100.4"), &h.ctx).await;
        assert_eq!(resp.status, 403);
        assert_ne!(String::from_utf8_lossy(&resp.body), "pong");
    });
}

#[test]
fn empty_allowlist_allows_any_ip() {
    block_on(async {
        let h = harness();
        make_collection(&h, "Open", serde_json::json!([])).await;
        assert_eq!(
            handle(ping_from("open", "198.51.100.9"), &h.ctx)
                .await
                .status,
            200
        );
    });
}

#[test]
fn invalid_allowlist_entry_is_rejected_on_save() {
    block_on(async {
        let h = harness();
        let ws = admin_json(
            &handle(
                areq(
                    &h,
                    Method::Post,
                    "/__admin/workspaces",
                    serde_json::json!({ "name": "Bad" }),
                ),
                &h.ctx,
            )
            .await,
        );
        let ws_id = ws["id"].as_str().unwrap().to_string();
        let resp = handle(
            areq(
                &h,
                Method::Post,
                &format!("/__admin/workspaces/{ws_id}/collections"),
                serde_json::json!({ "name": "Todo", "ipAllowlist": ["not-an-ip"] }),
            ),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 400);
    });
}
