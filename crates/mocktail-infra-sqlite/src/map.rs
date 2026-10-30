//! Entity <-> document mapping (SQL). Same domain entities as the native
//! target, guaranteeing identical document shape.

use crate::dto::*;
use mocktail_core::modules::auth::domain::{Membership, Role, User};
use mocktail_core::modules::collection::domain::{AllowOrigins, Collection, CorsPolicy, Source};
use mocktail_core::modules::folder::domain::Folder;
use mocktail_core::modules::mock::domain::{
    Fault, MatchKind, MatchTarget, Mock, Priority, RateLimit, RequestMatcher, ResponseSpec,
};
use mocktail_core::modules::request_log::domain::{RequestId, RequestLog, ResponseLog};
use mocktail_core::modules::workspace::domain::Workspace;
use mocktail_core::shared::error::{AppError, AppResult};
use mocktail_core::shared::http::Method;
use mocktail_core::shared::ids::{
    CollectionId, FolderId, Id, MembershipId, MockId, Slug, UserId, WorkspaceId,
};

fn corrupt(what: &str, detail: impl std::fmt::Display) -> AppError {
    AppError::Internal(format!("corrupt stored {what}: {detail}"))
}

// Workspace

pub fn workspace_to_doc(w: &Workspace) -> WorkspaceDoc {
    WorkspaceDoc {
        id: w.id.as_str().to_string(),
        name: w.name.clone(),
        slug: w.slug.as_str().to_string(),
        created_at: w.created_at,
    }
}

pub fn workspace_from_doc(d: WorkspaceDoc) -> AppResult<Workspace> {
    Ok(Workspace {
        id: WorkspaceId::new(d.id),
        name: d.name,
        slug: Slug::parse(&d.slug).map_err(|e| corrupt("workspace slug", e))?,
        created_at: d.created_at,
    })
}

// Collection

fn cors_to_doc(p: &CorsPolicy) -> CorsDoc {
    let (any, list) = match &p.allow_origins {
        AllowOrigins::Any => (true, Vec::new()),
        AllowOrigins::List(l) => (false, l.clone()),
    };
    CorsDoc {
        allowOriginsAny: any,
        allowOriginsList: list,
        allowMethods: p.allow_methods.clone(),
        allowHeaders: p.allow_headers.clone(),
        exposeHeaders: p.expose_headers.clone(),
        allowCredentials: p.allow_credentials,
        maxAgeSecs: p.max_age_secs,
    }
}

fn cors_from_doc(d: CorsDoc) -> CorsPolicy {
    CorsPolicy {
        allow_origins: if d.allowOriginsAny {
            AllowOrigins::Any
        } else {
            AllowOrigins::List(d.allowOriginsList)
        },
        allow_methods: d.allowMethods,
        allow_headers: d.allowHeaders,
        expose_headers: d.exposeHeaders,
        allow_credentials: d.allowCredentials,
        max_age_secs: d.maxAgeSecs,
    }
}

pub fn collection_to_doc(c: &Collection) -> CollectionDoc {
    CollectionDoc {
        id: c.id.as_str().to_string(),
        workspace_id: c.workspace_id.as_str().to_string(),
        name: c.name.clone(),
        slug: c.slug.as_str().to_string(),
        description: c.description.clone(),
        source: c.source.as_str().to_string(),
        base_path: c.base_path.clone(),
        cors: cors_to_doc(&c.cors_policy),
        version: c.version as i64,
        created_at: c.created_at,
    }
}

pub fn collection_from_doc(d: CollectionDoc) -> AppResult<Collection> {
    Ok(Collection {
        id: CollectionId::new(d.id),
        workspace_id: WorkspaceId::new(d.workspace_id),
        name: d.name,
        slug: Slug::parse(&d.slug).map_err(|e| corrupt("collection slug", e))?,
        description: d.description,
        source: Source::parse(&d.source).ok_or_else(|| corrupt("collection source", &d.source))?,
        base_path: d.base_path,
        cors_policy: cors_from_doc(d.cors),
        version: d.version.max(0) as u64,
        created_at: d.created_at,
    })
}

// Folder

pub fn folder_to_doc(f: &Folder) -> FolderDoc {
    FolderDoc {
        id: f.id.as_str().to_string(),
        workspace_id: f.workspace_id.as_str().to_string(),
        collection_id: f.collection_id.as_str().to_string(),
        name: f.name.clone(),
        parent_id: f.parent_id.as_ref().map(|id| id.as_str().to_string()),
        created_at: f.created_at,
    }
}

pub fn folder_from_doc(d: FolderDoc) -> AppResult<Folder> {
    Ok(Folder {
        id: FolderId::new(d.id),
        workspace_id: WorkspaceId::new(d.workspace_id),
        collection_id: CollectionId::new(d.collection_id),
        name: d.name,
        parent_id: d.parent_id.map(FolderId::new),
        created_at: d.created_at,
    })
}

// Mock

fn matcher_to_doc(m: &RequestMatcher) -> MatcherDoc {
    let (target_type, target_name) = match &m.target {
        MatchTarget::Query(n) => ("query", Some(n.clone())),
        MatchTarget::Header(n) => ("header", Some(n.clone())),
        MatchTarget::Body => ("body", None),
    };
    let (kind_type, value, path, equals) = match &m.kind {
        MatchKind::Exact(v) => ("exact", Some(v.clone()), None, None),
        MatchKind::Contains(v) => ("contains", Some(v.clone()), None, None),
        MatchKind::Regex(v) => ("regex", Some(v.clone()), None, None),
        MatchKind::JsonPath { path, equals } => {
            ("jsonpath", None, Some(path.clone()), equals.clone())
        }
    };
    MatcherDoc {
        targetType: target_type.to_string(),
        targetName: target_name,
        kindType: kind_type.to_string(),
        value,
        path,
        equals,
    }
}

fn matcher_from_doc(d: MatcherDoc) -> AppResult<RequestMatcher> {
    let target = match d.targetType.as_str() {
        "query" => MatchTarget::Query(
            d.targetName
                .clone()
                .ok_or_else(|| corrupt("matcher", "query target missing name"))?,
        ),
        "header" => MatchTarget::Header(
            d.targetName
                .clone()
                .ok_or_else(|| corrupt("matcher", "header target missing name"))?,
        ),
        "body" => MatchTarget::Body,
        other => return Err(corrupt("matcher target", other)),
    };
    let kind = match d.kindType.as_str() {
        "exact" => MatchKind::Exact(
            d.value
                .ok_or_else(|| corrupt("matcher", "exact missing value"))?,
        ),
        "contains" => MatchKind::Contains(
            d.value
                .ok_or_else(|| corrupt("matcher", "contains missing value"))?,
        ),
        "regex" => MatchKind::Regex(
            d.value
                .ok_or_else(|| corrupt("matcher", "regex missing value"))?,
        ),
        "jsonpath" => MatchKind::JsonPath {
            path: d
                .path
                .ok_or_else(|| corrupt("matcher", "jsonpath missing path"))?,
            equals: d.equals,
        },
        other => return Err(corrupt("matcher kind", other)),
    };
    Ok(RequestMatcher { target, kind })
}

pub fn mock_to_doc(m: &Mock) -> MockDoc {
    MockDoc {
        id: m.id.as_str().to_string(),
        workspace_id: m.workspace_id.as_str().to_string(),
        collection_id: m.collection_id.as_str().to_string(),
        folder_id: m.folder_id.as_ref().map(|id| id.as_str().to_string()),
        method: m.method.as_str().to_string(),
        path_template: m.path_template.clone(),
        matchers: m.matchers.iter().map(matcher_to_doc).collect(),
        response: ResponseDoc {
            status: m.response.status,
            headers: m.response.headers.clone(),
            body: String::from_utf8_lossy(&m.response.body).to_string(),
        },
        priority: m.priority.0,
        delay_ms: m.delay_ms,
        fault: m.fault.map(|f| match f {
            Fault::ConnectionReset => "connectionReset".to_string(),
            Fault::MalformedResponse => "malformedResponse".to_string(),
        }),
        rate_limit: m.rate_limit.map(|rl| RateLimitDoc {
            limit: rl.limit,
            windowSecs: rl.window_secs,
            status: rl.status,
        }),
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

pub fn mock_from_doc(d: MockDoc) -> AppResult<Mock> {
    let method = Method::parse(&d.method).ok_or_else(|| corrupt("mock method", &d.method))?;
    let matchers = d
        .matchers
        .into_iter()
        .map(matcher_from_doc)
        .collect::<AppResult<Vec<_>>>()?;
    let fault = match d.fault.as_deref() {
        Some("connectionReset") => Some(Fault::ConnectionReset),
        Some("malformedResponse") => Some(Fault::MalformedResponse),
        None => None,
        Some(other) => return Err(corrupt("mock fault", other)),
    };
    Ok(Mock {
        id: MockId::new(d.id),
        workspace_id: WorkspaceId::new(d.workspace_id),
        collection_id: CollectionId::new(d.collection_id),
        folder_id: d.folder_id.map(FolderId::new),
        method,
        path_template: d.path_template,
        matchers,
        response: ResponseSpec {
            status: d.response.status,
            headers: d.response.headers,
            body: d.response.body.into_bytes(),
        },
        priority: Priority(d.priority),
        delay_ms: d.delay_ms,
        fault,
        rate_limit: d.rate_limit.and_then(|rl| {
            // Legacy/disabled documents may carry limit 0.
            (rl.limit > 0).then_some(RateLimit {
                limit: rl.limit,
                window_secs: rl.windowSecs,
                status: rl.status,
            })
        }),
        created_at: d.created_at,
        updated_at: d.updated_at,
    })
}

// Request Log

pub fn log_to_doc(l: &RequestLog) -> RequestLogDoc {
    RequestLogDoc {
        id: l.id.as_str().to_string(),
        request_id: l.request_id.as_str().to_string(),
        workspace_id: l.workspace_id.as_str().to_string(),
        collection_id: l.collection_id.as_ref().map(|c| c.as_str().to_string()),
        received_at: l.received_at,
        method: l.method.clone(),
        path: l.path.clone(),
        request_headers: l.request_headers.clone(),
        request_body: l.request_body.clone(),
        matched_mock_id: l.matched_mock_id.as_ref().map(|m| m.as_str().to_string()),
        match_reason: l.match_reason.clone(),
        status: l.status,
        response: ResponseLogDoc {
            status: l.response.status,
            headers: l.response.headers.clone(),
            body: l.response.body.clone(),
        },
        ip: l.ip.clone(),
        country: l.country.clone(),
        lat: l.lat,
        lng: l.lng,
    }
}

pub fn log_from_doc(d: RequestLogDoc) -> AppResult<RequestLog> {
    let response_status = if d.response.status == 0 {
        d.status
    } else {
        d.response.status
    };
    Ok(RequestLog {
        id: Id::new(d.id),
        request_id: RequestId(d.request_id),
        workspace_id: WorkspaceId::new(d.workspace_id),
        collection_id: d.collection_id.map(CollectionId::new),
        received_at: d.received_at,
        method: d.method,
        path: d.path,
        request_headers: d.request_headers,
        request_body: d.request_body,
        matched_mock_id: d.matched_mock_id.map(MockId::new),
        match_reason: d.match_reason,
        status: d.status,
        response: ResponseLog {
            status: response_status,
            headers: d.response.headers,
            body: d.response.body,
        },
        ip: d.ip,
        country: d.country,
        lat: d.lat,
        lng: d.lng,
    })
}

// User

pub fn user_to_doc(u: &User) -> UserDoc {
    UserDoc {
        id: u.id.as_str().to_string(),
        username: u.username.clone(),
        display_name: u.display_name.clone(),
        created_at: u.created_at,
    }
}

pub fn user_from_doc(d: UserDoc) -> AppResult<User> {
    Ok(User {
        id: UserId::new(d.id),
        username: d.username,
        display_name: d.display_name,
        created_at: d.created_at,
    })
}

// Membership

pub fn membership_to_doc(m: &Membership) -> MembershipDoc {
    MembershipDoc {
        id: m.id.as_str().to_string(),
        user_id: m.user_id.as_str().to_string(),
        workspace_id: m.workspace_id.as_str().to_string(),
        role: m.role.as_str().to_string(),
    }
}

pub fn membership_from_doc(d: MembershipDoc) -> AppResult<Membership> {
    Ok(Membership {
        id: MembershipId::new(d.id),
        user_id: UserId::new(d.user_id),
        workspace_id: WorkspaceId::new(d.workspace_id),
        role: Role::parse(&d.role).ok_or_else(|| corrupt("membership role", &d.role))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_mock(rate_limit: serde_json::Value) -> MockDoc {
        serde_json::from_value(serde_json::json!({
            "id": "m1",
            "workspace_id": "w1",
            "collection_id": "c1",
            "method": "GET",
            "path_template": "/ping",
            "matchers": [],
            "response": {"status": 200, "headers": [], "body": "pong"},
            "priority": 0,
            "delay_ms": 0,
            "fault": null,
            "rate_limit": rate_limit,
            "created_at": 1,
            "updated_at": 2
        }))
        .unwrap()
    }

    #[test]
    fn legacy_rate_limit_documents_load() {
        let mock = mock_from_doc(legacy_mock(serde_json::json!({
            "enabled": true,
            "limit": 5,
            "windowSecs": 60,
            "scope": "ip",
            "key": "",
            "status": 429
        })))
        .unwrap();
        assert_eq!(
            mock.rate_limit,
            Some(RateLimit {
                limit: 5,
                window_secs: 60,
                status: 429
            })
        );

        // Missing folderId and a disabled legacy rate limit are both valid.
        let disabled = mock_from_doc(legacy_mock(serde_json::json!({"enabled": false}))).unwrap();
        assert!(disabled.folder_id.is_none());
        assert!(disabled.rate_limit.is_none());
    }

    #[test]
    fn legacy_request_log_documents_load() {
        let doc: RequestLogDoc = serde_json::from_value(serde_json::json!({
            "id": "l1",
            "request_id": "r1",
            "workspace_id": "w1",
            "collection_id": null,
            "received_at": 1,
            "method": "GET",
            "path": "/ping",
            "matched_mock_id": null,
            "match_reason": null,
            "status": 204
        }))
        .unwrap();
        let log = log_from_doc(doc).unwrap();
        assert!(log.request_headers.is_empty());
        assert_eq!(log.request_body, "");
        assert_eq!(log.response.status, 204);
        assert!(log.response.headers.is_empty());
        assert_eq!(log.response.body, "");
    }
}
