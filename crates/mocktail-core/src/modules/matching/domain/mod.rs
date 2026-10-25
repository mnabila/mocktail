//! Pure matching engine + precedence.
//!
//! No I/O, no async: takes a collection's mocks plus the (mount-stripped)
//! request, returns the selected mock, captured params, the reason it won, and
//! the losing candidates for the diagnostic log.

mod matcher;
mod path;
mod precedence;

pub use path::match_path;
pub use precedence::{MatchOutcome, resolve};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::mock::domain::{
        MatchKind, MatchTarget, Mock, Priority, RequestMatcher, ResponseSpec,
    };
    use crate::shared::http::{Headers, Method, Request};
    use crate::shared::ids::{CollectionId, MockId, WorkspaceId};

    fn req(method: Method, path: &str) -> Request {
        Request {
            method,
            host: "acme.mocktail.com".into(),
            path: path.into(),
            query: vec![],
            headers: Headers::new(),
            body: vec![],
            client_ip: Some("203.0.113.7".into()),
        }
    }

    fn mock(id: &str, method: Method, tmpl: &str) -> Mock {
        Mock {
            id: MockId::new(id),
            workspace_id: WorkspaceId::new("w"),
            collection_id: CollectionId::new("c"),
            folder_id: None,
            method,
            path_template: tmpl.into(),
            matchers: vec![],
            response: ResponseSpec {
                status: 200,
                headers: vec![],
                body: vec![],
            },
            priority: Priority(0),
            delay_ms: 0,
            fault: None,
            rate_limit: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn path_template_captures_params() {
        assert_eq!(
            match_path("/todos/{id}", "/todos/42"),
            Some(vec![("id".to_string(), "42".to_string())])
        );
        assert_eq!(match_path("/todos", "/todos"), Some(vec![]));
        assert_eq!(match_path("/todos/{id}", "/todos"), None);
        assert_eq!(match_path("/todos", "/todos/42"), None);
    }

    #[test]
    fn root_path() {
        assert_eq!(match_path("/", "/"), Some(vec![]));
    }

    #[test]
    fn literal_beats_param() {
        // Literal segments beat {param}.
        let mocks = vec![
            mock("param", Method::Get, "/todos/{id}"),
            mock("literal", Method::Get, "/todos/special"),
        ];
        let out = resolve(
            &mocks,
            &req(Method::Get, "/todos/special"),
            "/todos/special",
        )
        .unwrap();
        assert_eq!(out.mock.id.as_str(), "literal");
        assert!(out.reason.contains("more specific"));
    }

    #[test]
    fn explicit_priority_wins() {
        // Explicit priority beats specificity.
        let mut hi = mock("hi", Method::Get, "/todos/{id}");
        hi.priority = Priority(10);
        let lo = mock("lo", Method::Get, "/todos/special");
        let mocks = vec![lo, hi];
        let out = resolve(
            &mocks,
            &req(Method::Get, "/todos/special"),
            "/todos/special",
        )
        .unwrap();
        assert_eq!(out.mock.id.as_str(), "hi");
        assert!(out.reason.contains("higher priority"));
    }

    #[test]
    fn most_recently_updated_breaks_tie() {
        let mut old = mock("old", Method::Get, "/todos/{id}");
        old.updated_at = 100;
        let mut new = mock("new", Method::Get, "/todos/{id}");
        new.updated_at = 200;
        let mocks = vec![old, new];
        let out = resolve(&mocks, &req(Method::Get, "/todos/9"), "/todos/9").unwrap();
        assert_eq!(out.mock.id.as_str(), "new");
        assert!(out.reason.contains("recently updated"));
    }

    #[test]
    fn method_must_match() {
        let mocks = vec![mock("m", Method::Post, "/todos")];
        assert!(resolve(&mocks, &req(Method::Get, "/todos"), "/todos").is_none());
    }

    #[test]
    fn query_matcher() {
        let mut m = mock("m", Method::Get, "/todos");
        m.matchers = vec![RequestMatcher {
            target: MatchTarget::Query("status".into()),
            kind: MatchKind::Exact("done".into()),
        }];
        let mocks = vec![m];
        let mut r = req(Method::Get, "/todos");
        r.query = vec![("status".into(), "done".into())];
        assert!(resolve(&mocks, &r, "/todos").is_some());

        let mut r2 = req(Method::Get, "/todos");
        r2.query = vec![("status".into(), "open".into())];
        assert!(resolve(&mocks, &r2, "/todos").is_none());
    }

    #[test]
    fn jsonpath_body_matcher() {
        let mut m = mock("m", Method::Post, "/todos");
        m.matchers = vec![RequestMatcher {
            target: MatchTarget::Body,
            kind: MatchKind::JsonPath {
                path: "user.role".into(),
                equals: Some("admin".into()),
            },
        }];
        let mocks = vec![m];
        let mut r = req(Method::Post, "/todos");
        r.body = br#"{"user":{"role":"admin"}}"#.to_vec();
        assert!(resolve(&mocks, &r, "/todos").is_some());

        let mut r2 = req(Method::Post, "/todos");
        r2.body = br#"{"user":{"role":"guest"}}"#.to_vec();
        assert!(resolve(&mocks, &r2, "/todos").is_none());
    }
}
