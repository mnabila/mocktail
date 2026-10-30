//! Conversion between axum/http and the core's framework-free HTTP types.

use axum::body::{Body, to_bytes};
use axum::extract::FromRequest;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response as AxumResponse};
use mocktail_core::shared::http::{Headers, Method, Request as CoreRequest, Response};

/// Adapter-provided peer IP (native inserts it from the socket connection).
#[derive(Clone)]
pub struct ClientIp(pub String);

/// Axum extractor that buffers an inbound request into the core request type.
pub struct AxumRequest(pub CoreRequest);

impl<S> FromRequest<S> for AxumRequest
where
    S: Send + Sync,
{
    type Rejection = AxumResponse;

    async fn from_request(
        req: axum::extract::Request,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        // Cap inbound bodies so an unauthenticated client can't OOM the server.
        const MAX_BODY: usize = 16 * 1024 * 1024;
        let body = to_bytes(body, MAX_BODY)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;

        let mut headers = Headers::new();
        for (name, value) in &parts.headers {
            headers.insert(
                name.as_str().to_string(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            );
        }

        let host = parts
            .headers
            .get(axum::http::header::HOST)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
            .or_else(|| parts.uri.host().map(str::to_string))
            .unwrap_or_default();
        let query = parts
            .uri
            .query()
            .map(|q| {
                form_urlencoded::parse(q.as_bytes())
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect()
            })
            .unwrap_or_default();
        let client_ip = parts
            .extensions
            .get::<ClientIp>()
            .map(|ip| ip.0.clone())
            .or_else(|| {
                headers
                    .get("cf-connecting-ip")
                    .map(str::trim)
                    .filter(|ip| !ip.is_empty())
                    .map(str::to_string)
                    .or_else(|| {
                        headers
                            .get("x-forwarded-for")
                            .and_then(|v| v.split(',').next())
                            .map(str::trim)
                            .filter(|ip| !ip.is_empty())
                            .map(str::to_string)
                    })
                    .or_else(|| {
                        headers
                            .get("x-real-ip")
                            .map(str::trim)
                            .filter(|ip| !ip.is_empty())
                            .map(str::to_string)
                    })
            });

        Ok(AxumRequest(CoreRequest {
            method: Method::parse(parts.method.as_str()).unwrap_or(Method::Get),
            host,
            path: parts.uri.path().to_string(),
            query,
            headers,
            body: body.to_vec(),
            client_ip,
        }))
    }
}

/// Build an axum request for the test/back-compat `handle` convenience.
pub fn to_axum_request(req: CoreRequest) -> axum::extract::Request {
    let query = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(req.query.iter())
        .finish();
    let path_and_query = if query.is_empty() {
        req.path
    } else {
        format!("{}?{query}", req.path)
    };
    let uri: Uri = path_and_query
        .parse()
        .unwrap_or_else(|_| Uri::from_static("/"));
    let mut builder = axum::http::Request::builder()
        .method(req.method.as_str())
        .uri(uri)
        .header(axum::http::header::HOST, req.host);
    for (name, value) in req.headers.iter() {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            builder = builder.header(name, value);
        }
    }
    builder.body(Body::from(req.body)).unwrap_or_default()
}

/// Convert a core response into an axum response.
pub fn to_axum(resp: Response) -> AxumResponse {
    let mut out = AxumResponse::new(Body::from(resp.body));
    *out.status_mut() =
        StatusCode::from_u16(resp.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    for (name, value) in resp.headers.iter() {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(value),
        ) {
            out.headers_mut().append(name, value);
        }
    }
    out
}

/// Convert an axum response back to the core type (used by `handle` tests).
pub async fn from_axum(resp: AxumResponse) -> Response {
    let status = resp.status().as_u16();
    let headers = headers_from_map(resp.headers());
    let body = to_bytes(resp.into_body(), usize::MAX)
        .await
        .map(|b| b.to_vec())
        .unwrap_or_default();
    Response {
        status,
        headers,
        body,
    }
}

fn headers_from_map(map: &HeaderMap) -> Headers {
    Headers::from_pairs(map.iter().map(|(k, v)| {
        (
            k.as_str().to_string(),
            String::from_utf8_lossy(v.as_bytes()).into_owned(),
        )
    }))
}
