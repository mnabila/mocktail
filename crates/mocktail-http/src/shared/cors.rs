//! CORS: automatic preflight + response augmentation.

use mocktail_core::modules::collection::domain::CorsPolicy;
use mocktail_core::shared::http::{Request, Response};

/// Build the automatic `OPTIONS` preflight response from a collection policy.
pub fn preflight_response(policy: &CorsPolicy, req: &Request) -> Response {
    let origin = req.headers.get("origin");
    let mut resp = Response::new(204);
    apply_cors_headers(&mut resp, policy, origin);
    resp.headers.insert(
        "Access-Control-Allow-Methods",
        policy.allow_methods.join(", "),
    );
    resp.headers.insert(
        "Access-Control-Allow-Headers",
        policy.allow_headers.join(", "),
    );
    resp.headers
        .insert("Access-Control-Max-Age", policy.max_age_secs.to_string());
    resp
}

/// Add CORS response headers to an actual (non-preflight) mock response.
pub fn apply_cors_headers(resp: &mut Response, policy: &CorsPolicy, origin: Option<&str>) {
    if let Some(allow) = policy.resolve_allow_origin(origin) {
        resp.headers.insert("Access-Control-Allow-Origin", allow);
        // When the allowed origin is reflected (not `*`), caches must vary.
        if origin.is_some() {
            resp.headers.insert("Vary", "Origin");
        }
    }
    if policy.allow_credentials {
        resp.headers
            .insert("Access-Control-Allow-Credentials", "true");
    }
    if !policy.expose_headers.is_empty() {
        resp.headers.insert(
            "Access-Control-Expose-Headers",
            policy.expose_headers.join(", "),
        );
    }
}
