//! Mock (stub) entity and its matcher/response value objects.
//! Pure data + invariants; matching logic lives in the matching feature.

use crate::shared::http::Method;
use crate::shared::ids::{CollectionId, FolderId, MockId, WorkspaceId};
use crate::shared::ports::UnixMillis;

/// What part of the request a matcher inspects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchTarget {
    Query(String),
    Header(String),
    /// Whole request body (as UTF-8 text).
    Body,
}

/// How a matcher compares the target value.
#[derive(Clone, Debug)]
pub enum MatchKind {
    Exact(String),
    Contains(String),
    Regex(String),
    /// JSONPath-ish lookup into a JSON body: a `/`-separated pointer and an
    /// optional expected value. See matching engine for evaluation.
    JsonPath {
        path: String,
        equals: Option<String>,
    },
}

impl PartialEq for MatchKind {
    fn eq(&self, other: &Self) -> bool {
        use MatchKind::*;
        match (self, other) {
            (Exact(a), Exact(b)) => a == b,
            (Contains(a), Contains(b)) => a == b,
            (Regex(a), Regex(b)) => a == b,
            (
                JsonPath {
                    path: p1,
                    equals: e1,
                },
                JsonPath {
                    path: p2,
                    equals: e2,
                },
            ) => p1 == p2 && e1 == e2,
            _ => false,
        }
    }
}
impl Eq for MatchKind {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestMatcher {
    pub target: MatchTarget,
    pub kind: MatchKind,
}

/// A simulated fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// Drop the connection with no response.
    ConnectionReset,
    /// Return malformed data instead of the configured body.
    MalformedResponse,
}

/// Forward (proxy) the matched request to an external API instead of returning
/// the static body. The sub-path after the collection mount and the query
/// string are appended to `base_url`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForwardSpec {
    pub base_url: String,
}

/// Reject a forward target that could be used for SSRF against internal
/// infrastructure: require an http(s) scheme and block literal
/// private/loopback/link-local hosts (incl. the cloud-metadata address
/// 169.254.169.254). Called at the trust boundary when a mock is saved.
///
/// shortcut: a hostname that *resolves* to a private IP (DNS-based SSRF /
/// rebinding) is not caught here - that needs runtime resolution, which is not
/// available in this wasm-clean layer. Pair with network egress controls or a
/// host allowlist to close that gap.
pub fn validate_forward_base_url(url: &str) -> Result<(), String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| "forward.baseUrl must start with http:// or https://".to_string())?;
    // Authority is everything up to the first path/query/fragment delimiter.
    let authority = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    // Drop any userinfo, then extract the host (handling [ipv6]:port and host:port).
    let hostport = authority.rsplit('@').next().unwrap_or(authority);
    let host = if let Some(after) = hostport.strip_prefix('[') {
        after.split(']').next().unwrap_or("")
    } else {
        hostport.split(':').next().unwrap_or(hostport)
    };
    if host.is_empty() {
        return Err("forward.baseUrl is missing a host".into());
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>()
        && is_blocked_ip(&ip)
    {
        return Err(format!(
            "forward.baseUrl host {host} is a private/loopback address"
        ));
    }
    Ok(())
}

fn is_blocked_ip(ip: &std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
        }
        IpAddr::V6(v6) => {
            let seg = v6.segments();
            v6.is_loopback()
                || v6.is_unspecified()
                || (seg[0] & 0xfe00) == 0xfc00 // fc00::/7 unique-local
                || (seg[0] & 0xffc0) == 0xfe80 // fe80::/10 link-local
                || v6
                    .to_ipv4_mapped()
                    .is_some_and(|v4| is_blocked_ip(&IpAddr::V4(v4)))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResponseSpec {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    /// Response body; may contain `{{param}}` templating placeholders filled
    /// from captured path params.
    pub body: Vec<u8>,
    /// When set, the request is proxied to this upstream and `status`/`headers`/
    /// `body`/`fault` are ignored for this mock.
    pub forward: Option<ForwardSpec>,
}

/// Explicit integer priority; higher wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Priority(pub i32);

/// Per-endpoint request throttle. Only the client IP is used as the counter
/// key; the fixed window is implementation-defined (per process/isolate).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateLimit {
    pub limit: u32,
    pub window_secs: u32,
    /// HTTP status returned when the limit is exceeded (usually 429).
    pub status: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mock {
    pub id: MockId,
    pub workspace_id: WorkspaceId,
    pub collection_id: CollectionId,
    /// Optional grouping folder inside the collection.
    pub folder_id: Option<FolderId>,
    pub method: Method,
    /// Spec-relative path template, e.g. `/users/{id}`.
    pub path_template: String,
    pub matchers: Vec<RequestMatcher>,
    pub response: ResponseSpec,
    pub priority: Priority,
    pub delay_ms: u32,
    pub fault: Option<Fault>,
    pub rate_limit: Option<RateLimit>,
    pub created_at: UnixMillis,
    pub updated_at: UnixMillis,
}

#[cfg(test)]
mod forward_url_tests {
    use super::validate_forward_base_url as v;

    #[test]
    fn accepts_public_hosts() {
        assert!(v("https://api.example.com").is_ok());
        assert!(v("http://example.com/base/path").is_ok());
        assert!(v("https://203.0.113.9:8443/x").is_ok());
    }

    #[test]
    fn rejects_bad_scheme_or_missing_host() {
        assert!(v("ftp://example.com").is_err());
        assert!(v("example.com").is_err());
        assert!(v("https://").is_err());
    }

    #[test]
    fn rejects_internal_ip_literals() {
        for u in [
            "http://169.254.169.254/latest/meta-data/", // cloud metadata
            "http://127.0.0.1",
            "http://10.0.0.5",
            "http://192.168.1.1:9000",
            "http://172.16.0.1",
            "http://[::1]",
            "http://[fd00::1]/x",
            "http://[fe80::1]",
            "http://user:pass@127.0.0.1/x", // userinfo must not hide the host
        ] {
            assert!(v(u).is_err(), "should reject {u}");
        }
    }
}
