//! Forward/proxy response mode: content-routed mocks forward the request to an
//! external API, preserving sub-path + query and stripping sensitive headers.

mod common;

use async_trait::async_trait;
use common::{Harness, admin_json, areq, block_on, harness_with_http, mock_req};
use mocktail_core::shared::http::Method;
use mocktail_core::shared::ports::{HttpClient, HttpResponse};
use mocktail_http::handle;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct Recorded {
    method: String,
    url: String,
    body: Vec<u8>,
    headers: Vec<(String, String)>,
}

/// Records every forwarded request and echoes the upstream URL back in the body
/// so the test can assert which upstream was hit and that it survived the trip.
struct RecordingHttp {
    calls: Arc<Mutex<Vec<Recorded>>>,
}

#[async_trait]
impl HttpClient for RecordingHttp {
    async fn post_form(
        &self,
        _url: &str,
        _form: &[(String, String)],
        _headers: &[(String, String)],
    ) -> Result<HttpResponse, String> {
        Err("unused".into())
    }
    async fn get(&self, _url: &str, _headers: &[(String, String)]) -> Result<HttpResponse, String> {
        Err("unused".into())
    }
    async fn post_json(
        &self,
        _url: &str,
        _body: &[u8],
        _headers: &[(String, String)],
    ) -> Result<HttpResponse, String> {
        Err("unused".into())
    }
    async fn request(
        &self,
        method: &str,
        url: &str,
        body: &[u8],
        headers: &[(String, String)],
    ) -> Result<HttpResponse, String> {
        self.calls.lock().unwrap().push(Recorded {
            method: method.to_string(),
            url: url.to_string(),
            body: body.to_vec(),
            headers: headers.to_vec(),
        });
        Ok(HttpResponse {
            status: 201,
            headers: vec![
                ("content-type".into(), "application/json".into()),
                // A hop-by-hop header the forward path must NOT copy back.
                ("transfer-encoding".into(), "chunked".into()),
                // Upstream-controlled headers that must NOT reach the client:
                // cookie fixation + CORS override.
                ("set-cookie".into(), "sid=evil; Path=/".into()),
                (
                    "access-control-allow-origin".into(),
                    "https://evil.test".into(),
                ),
            ],
            body: format!("upstream:{url}").into_bytes(),
        })
    }
}

async fn setup_collection(h: &Harness) -> String {
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

async fn add_forward_mock(h: &Harness, coll_id: &str, needle: &str, base_url: &str) {
    handle(
        areq(
            h,
            Method::Post,
            &format!("/__admin/collections/{coll_id}/mocks"),
            serde_json::json!({
                "method": "POST",
                "pathTemplate": "/hello",
                "matchers": [{
                    "target": {"type": "body"},
                    "kind": {"type": "contains", "value": needle}
                }],
                "response": {"status": 200, "body": "", "forward": {"baseUrl": base_url}}
            }),
        ),
        &h.ctx,
    )
    .await;
}

#[test]
fn forwards_matched_request_and_strips_sensitive_headers() {
    block_on(async {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let h = harness_with_http(Arc::new(RecordingHttp {
            calls: calls.clone(),
        }));
        let coll_id = setup_collection(&h).await;
        add_forward_mock(&h, &coll_id, "xxx", "https://xxx-api.test").await;

        // POST /todo/hello?q=1 with a body containing "xxx" and sensitive headers.
        let mut req = mock_req("acme", "/todo/hello");
        req.method = Method::Post;
        req.query = vec![("q".into(), "1".into())];
        req.body = b"please route xxx here".to_vec();
        req.headers.insert("authorization", "Bearer secret");
        req.headers.insert("cookie", "mocktail_session=leak");
        req.headers.insert("x-custom", "keep-me");
        let resp = handle(req, &h.ctx).await;

        // Upstream response flows back to the client (status + body).
        assert_eq!(resp.status, 201);
        assert_eq!(
            String::from_utf8_lossy(&resp.body),
            "upstream:https://xxx-api.test/hello?q=1"
        );
        // Hop-by-hop header from the upstream is not copied back.
        assert_eq!(resp.headers.get("transfer-encoding"), None);
        assert_eq!(resp.headers.get("content-type"), Some("application/json"));
        // Upstream Set-Cookie / CORS headers must not be relayed to the client.
        assert_eq!(resp.headers.get("set-cookie"), None);
        assert_ne!(
            resp.headers.get("access-control-allow-origin"),
            Some("https://evil.test")
        );

        // Exactly one upstream call, with the right method/url/body.
        let recorded = calls.lock().unwrap().clone();
        assert_eq!(recorded.len(), 1);
        let c = &recorded[0];
        assert_eq!(c.method, "POST");
        assert_eq!(c.url, "https://xxx-api.test/hello?q=1");
        assert_eq!(c.body, b"please route xxx here");
        // Sensitive headers stripped; harmless one forwarded.
        let names: Vec<String> = c.headers.iter().map(|(k, _)| k.to_lowercase()).collect();
        assert!(names.contains(&"x-custom".to_string()));
        assert!(!names.contains(&"authorization".to_string()));
        assert!(!names.contains(&"cookie".to_string()));
    });
}

#[test]
fn content_routes_to_the_matching_upstream() {
    block_on(async {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let h = harness_with_http(Arc::new(RecordingHttp {
            calls: calls.clone(),
        }));
        let coll_id = setup_collection(&h).await;
        add_forward_mock(&h, &coll_id, "xxx", "https://xxx-api.test").await;
        add_forward_mock(&h, &coll_id, "yyy", "https://yyy-api.test").await;

        // Body contains "yyy" -> routed to the yyy upstream.
        let mut req = mock_req("acme", "/todo/hello");
        req.method = Method::Post;
        req.body = b"send yyy over".to_vec();
        let resp = handle(req, &h.ctx).await;
        assert_eq!(
            String::from_utf8_lossy(&resp.body),
            "upstream:https://yyy-api.test/hello"
        );

        // A body matching neither needle falls through to a 404 (no forward).
        let mut miss = mock_req("acme", "/todo/hello");
        miss.method = Method::Post;
        miss.body = b"nothing here".to_vec();
        let resp = handle(miss, &h.ctx).await;
        assert_eq!(resp.status, 404);
        // Only the one matched request reached an upstream.
        assert_eq!(calls.lock().unwrap().len(), 1);
    });
}

#[test]
fn rejects_forward_to_internal_address() {
    block_on(async {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let h = harness_with_http(Arc::new(RecordingHttp {
            calls: calls.clone(),
        }));
        let coll_id = setup_collection(&h).await;
        // Pointing a forward target at the cloud-metadata address must be
        // rejected at save time (SSRF guard), not silently accepted.
        let resp = handle(
            areq(
                &h,
                Method::Post,
                &format!("/__admin/collections/{coll_id}/mocks"),
                serde_json::json!({
                    "method": "GET",
                    "pathTemplate": "/x",
                    "response": {"status": 200, "body": "",
                        "forward": {"baseUrl": "http://169.254.169.254/latest/meta-data/"}}
                }),
            ),
            &h.ctx,
        )
        .await;
        assert_eq!(resp.status, 400);
    });
}
