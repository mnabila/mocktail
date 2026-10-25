//! Framework-free HTTP types the runtime adapters convert to/from.

use std::fmt;

pub type Bytes = Vec<u8>;

/// HTTP methods mocktail matches on. `Other` keeps unknown verbs routable.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Trace,
    Connect,
}

impl Method {
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
            Method::Trace => "TRACE",
            Method::Connect => "CONNECT",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.to_ascii_uppercase().as_str() {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "PATCH" => Method::Patch,
            "DELETE" => Method::Delete,
            "HEAD" => Method::Head,
            "OPTIONS" => Method::Options,
            "TRACE" => Method::Trace,
            "CONNECT" => Method::Connect,
            _ => return None,
        })
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Case-insensitive header multimap, preserving the original insertion name.
#[derive(Clone, Default, Debug)]
pub struct Headers(Vec<(String, String)>);

impl Headers {
    pub fn new() -> Self {
        Headers(Vec::new())
    }

    pub fn from_pairs(pairs: impl IntoIterator<Item = (String, String)>) -> Self {
        Headers(pairs.into_iter().collect())
    }

    /// First value for `name`, matched case-insensitively.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.0.push((name.into(), value.into()));
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A normalized inbound request. `host` resolves apex-admin vs the
/// `{workspace}.apex` mock surface.
#[derive(Clone, Debug)]
pub struct Request {
    pub method: Method,
    pub host: String,
    pub path: String,
    pub query: Vec<(String, String)>,
    pub headers: Headers,
    pub body: Bytes,
    /// Peer/client address resolved by the runtime adapter. Native uses the
    /// socket peer; the Worker uses Cloudflare's forwarding headers.
    pub client_ip: Option<String>,
}

impl Request {
    pub fn query_first(&self, key: &str) -> Option<&str> {
        self.query
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// A normalized outbound response.
#[derive(Clone, Debug)]
pub struct Response {
    pub status: u16,
    pub headers: Headers,
    pub body: Bytes,
}

impl Response {
    pub fn new(status: u16) -> Self {
        Response {
            status,
            headers: Headers::new(),
            body: Vec::new(),
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name, value);
        self
    }

    pub fn with_body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }

    /// Build a JSON response, setting `content-type` and the body.
    pub fn json(status: u16, value: &serde_json::Value) -> Self {
        let body = serde_json::to_vec(value).unwrap_or_default();
        Response::new(status)
            .with_header("content-type", "application/json")
            .with_body(body)
    }

    /// Build a dashboard API response using the shared envelope:
    /// `{ status: { code, message }, data, metadata }`.
    pub fn api_json(status: u16, data: &serde_json::Value) -> Self {
        let body = serde_json::json!({
            "status": {
                "code": api_status_code(status),
                "message": api_status_message(status),
            },
            "data": data,
            "metadata": {},
        });
        Response::json(status, &body)
    }
}

/// Dashboard envelope code: 3-digit HTTP status + service code `000` +
/// sequence `0`, e.g. `200` becomes `"2000000"`.
pub fn api_status_code(status: u16) -> String {
    format!("{status:03}0000")
}

/// Standard reason phrase used in the envelope's `status.message`.
pub fn api_status_message(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Error",
    }
}
