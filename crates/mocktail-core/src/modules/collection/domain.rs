//! Collection aggregate: grouping, mount path, CORS policy.

use crate::shared::ids::{CollectionId, Slug, WorkspaceId};
use crate::shared::ports::UnixMillis;

/// Where a collection's mocks came from. Indistinguishable to the runtime —
/// this is metadata only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    OpenApi,
    Manual,
}

impl Source {
    pub fn as_str(&self) -> &'static str {
        match self {
            Source::OpenApi => "openapi",
            Source::Manual => "manual",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "openapi" => Some(Source::OpenApi),
            "manual" => Some(Source::Manual),
            _ => None,
        }
    }
}

/// Per-collection CORS policy. Default is browser-usable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorsPolicy {
    /// Allowed origins: `*` (any) or an exact list.
    pub allow_origins: AllowOrigins,
    pub allow_methods: Vec<String>,
    pub allow_headers: Vec<String>,
    pub expose_headers: Vec<String>,
    pub allow_credentials: bool,
    pub max_age_secs: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AllowOrigins {
    Any,
    List(Vec<String>),
}

impl Default for CorsPolicy {
    fn default() -> Self {
        CorsPolicy {
            allow_origins: AllowOrigins::Any,
            allow_methods: ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            allow_headers: vec!["*".to_string()],
            expose_headers: vec!["X-Request-Id".to_string()],
            allow_credentials: false,
            max_age_secs: 600,
        }
    }
}

impl CorsPolicy {
    /// Resolve the `Access-Control-Allow-Origin` value for a request origin.
    /// `*` with credentials must echo the concrete origin, not `*`.
    pub fn resolve_allow_origin(&self, request_origin: Option<&str>) -> Option<String> {
        match &self.allow_origins {
            AllowOrigins::Any => {
                if self.allow_credentials {
                    request_origin.map(|o| o.to_string())
                } else {
                    Some("*".to_string())
                }
            }
            AllowOrigins::List(list) => {
                let origin = request_origin?;
                if list.iter().any(|o| o == origin) {
                    Some(origin.to_string())
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Collection {
    pub id: CollectionId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub slug: Slug,
    pub description: String,
    pub source: Source,
    /// Spec-relative base path, e.g. `/api/v1`. May be empty.
    pub base_path: String,
    pub cors_policy: CorsPolicy,
    /// Bumped on every edit to invalidate the mock-set cache.
    pub version: u64,
    pub created_at: UnixMillis,
}

impl Collection {
    /// The mount path = `/{slug}{base_path}`.
    pub fn mount_path(&self) -> String {
        format!("/{}{}", self.slug, self.base_path)
    }

    /// Strip the mount path from an incoming (subdomain-local) path, returning
    /// the spec-relative remainder the matcher runs against.
    /// Returns `None` if `path` is not under this collection's mount.
    pub fn strip_mount<'a>(&self, path: &'a str) -> Option<&'a str> {
        let mount = self.mount_path();
        let rest = path.strip_prefix(&mount)?;
        // The boundary must be a path separator or the end, so `/todos` does
        // not match a collection mounted at `/todo`.
        if rest.is_empty() {
            Some("/")
        } else if rest.starts_with('/') {
            Some(rest)
        } else {
            None
        }
    }
}

/// Normalize a base path: empty stays empty; otherwise ensure a leading slash
/// and strip a trailing slash so `mount_path` composes cleanly.
pub fn normalize_base_path(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return String::new();
    }
    let with_lead = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    };
    with_lead.trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::ids::Slug;

    fn coll(slug: &str, base: &str) -> Collection {
        Collection {
            id: CollectionId::new("c1"),
            workspace_id: WorkspaceId::new("w1"),
            name: "todo".into(),
            slug: Slug::parse(slug).unwrap(),
            description: "".into(),
            source: Source::Manual,
            base_path: normalize_base_path(base),
            cors_policy: CorsPolicy::default(),
            version: 0,
            created_at: 0,
        }
    }

    #[test]
    fn mount_and_strip() {
        let c = coll("todo", "/api/v1");
        assert_eq!(c.mount_path(), "/todo/api/v1");
        assert_eq!(c.strip_mount("/todo/api/v1/todos/42"), Some("/todos/42"));
        assert_eq!(c.strip_mount("/todo/api/v1"), Some("/"));
        assert_eq!(c.strip_mount("/other/x"), None);
    }

    #[test]
    fn strip_respects_segment_boundary() {
        let c = coll("todo", "");
        assert_eq!(c.mount_path(), "/todo");
        // `/todos` must not be treated as under mount `/todo`.
        assert_eq!(c.strip_mount("/todos"), None);
        assert_eq!(c.strip_mount("/todo/x"), Some("/x"));
    }

    #[test]
    fn base_path_normalized() {
        assert_eq!(normalize_base_path(""), "");
        assert_eq!(normalize_base_path("/"), "");
        assert_eq!(normalize_base_path("api/v1"), "/api/v1");
        assert_eq!(normalize_base_path("/api/v1/"), "/api/v1");
    }

    #[test]
    fn cors_any_without_credentials_is_star() {
        let p = CorsPolicy::default();
        assert_eq!(
            p.resolve_allow_origin(Some("https://x.com")),
            Some("*".to_string())
        );
    }

    #[test]
    fn cors_list_echoes_only_allowed() {
        let p = CorsPolicy {
            allow_origins: AllowOrigins::List(vec!["https://a.com".into()]),
            ..CorsPolicy::default()
        };
        assert_eq!(
            p.resolve_allow_origin(Some("https://a.com")),
            Some("https://a.com".to_string())
        );
        assert_eq!(p.resolve_allow_origin(Some("https://b.com")), None);
    }
}
