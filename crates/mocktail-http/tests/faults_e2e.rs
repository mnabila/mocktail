//! Fault + delay + templating enactment on the mock runtime.

mod common;

use common::{admin_json, areq, block_on, harness, mock_req};
use mocktail_core::shared::http::Method;
use mocktail_http::handle;
use mocktail_http::handle::FAULT_HEADER;

async fn setup_collection(h: &common::Harness) -> String {
    let ws = admin_json(
        &handle(
            areq(
                h,
                Method::Post,
                "/__admin/workspaces",
                serde_json::json!({"name": "Acme"}),
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
                serde_json::json!({"name": "Todo", "basePath": ""}),
            ),
            &h.ctx,
        )
        .await,
    );
    coll["id"].as_str().unwrap().to_string()
}

#[test]
fn malformed_fault_and_templating() {
    block_on(async {
        let h = harness();
        let coll_id = setup_collection(&h).await;
        // Mock with a request-templated body AND a malformed fault.
        handle(
            areq(
                &h,
                Method::Post,
                &format!("/__admin/collections/{coll_id}/mocks"),
                serde_json::json!({
                    "method": "GET",
                    "pathTemplate": "/u/{id}",
                    "response": {"status": 200, "body": "id={{id}} t={{query.t}}"},
                    "fault": "malformedResponse"
                }),
            ),
            &h.ctx,
        )
        .await;

        let mut req = mock_req("acme", "/todo/u/7");
        req.query = vec![("t".into(), "x".into())];
        let resp = handle(req, &h.ctx).await;
        // Malformed fault overrides the (templated) body with garbage.
        assert_eq!(resp.headers.get(FAULT_HEADER), Some("malformed-response"));
        assert!(String::from_utf8_lossy(&resp.body).contains("malformed response fault"));
    });
}

#[test]
fn connection_reset_fault() {
    block_on(async {
        let h = harness();
        let coll_id = setup_collection(&h).await;
        handle(
            areq(
                &h,
                Method::Post,
                &format!("/__admin/collections/{coll_id}/mocks"),
                serde_json::json!({
                    "method": "GET",
                    "pathTemplate": "/boom",
                    "response": {"status": 200, "body": "ok"},
                    "fault": "connectionReset"
                }),
            ),
            &h.ctx,
        )
        .await;

        let resp = handle(mock_req("acme", "/todo/boom"), &h.ctx).await;
        // The shared HTTP layer represents the fault as 502 + marker.
        assert_eq!(resp.status, 502);
        assert_eq!(resp.headers.get(FAULT_HEADER), Some("connection-reset"));
    });
}

#[test]
fn templating_without_fault() {
    block_on(async {
        let h = harness();
        let coll_id = setup_collection(&h).await;
        handle(
            areq(
                &h,
                Method::Post,
                &format!("/__admin/collections/{coll_id}/mocks"),
                serde_json::json!({
                    "method": "GET",
                    "pathTemplate": "/u/{id}",
                    "response": {"status": 200, "body": "{\"id\":\"{{id}}\",\"h\":\"{{header.x-tenant}}\"}"}
                }),
            ),
            &h.ctx,
        )
        .await;
        let mut req = mock_req("acme", "/todo/u/9");
        req.headers.insert("X-Tenant", "acme-co");
        let resp = handle(req, &h.ctx).await;
        assert_eq!(resp.status, 200);
        assert_eq!(
            String::from_utf8_lossy(&resp.body),
            "{\"id\":\"9\",\"h\":\"acme-co\"}"
        );
    });
}
