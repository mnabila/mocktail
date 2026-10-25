//! Response body templating.

use crate::shared::http::Request;

/// Render a response body, substituting `{{token}}` placeholders from the
/// matched request. Supported tokens:
/// `{{name}}` / `{{path.name}}` (captured path params), `{{query.name}}`,
/// `{{header.name}}`, and `{{body}}` (the raw request body text). An unknown
/// token resolves to an empty string. Non-UTF-8 bodies are returned unchanged.
pub fn render_body(body: &[u8], params: &[(String, String)], req: &Request) -> Vec<u8> {
    let Ok(text) = std::str::from_utf8(body) else {
        return body.to_vec();
    };
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find("}}") {
            let token = after[..end].trim();
            out.push_str(&resolve_token(token, params, req));
            rest = &after[end + 2..];
        } else {
            // Unterminated placeholder: emit the rest verbatim.
            out.push_str(&rest[start..]);
            rest = "";
            break;
        }
    }
    out.push_str(rest);
    out.into_bytes()
}

fn resolve_token(token: &str, params: &[(String, String)], req: &Request) -> String {
    let param = |name: &str| {
        params
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    if let Some(name) = token.strip_prefix("query.") {
        req.query_first(name).unwrap_or("").to_string()
    } else if let Some(name) = token.strip_prefix("header.") {
        req.headers.get(name).unwrap_or("").to_string()
    } else if let Some(name) = token.strip_prefix("path.") {
        param(name)
    } else if token == "body" {
        String::from_utf8_lossy(&req.body).into_owned()
    } else {
        // Bare token = captured path param (back-compatible).
        param(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::http::{Headers, Method};

    fn req() -> Request {
        let mut headers = Headers::new();
        headers.insert("X-Tenant", "acme");
        Request {
            method: Method::Post,
            host: "acme.mocktail.com".into(),
            path: "/x".into(),
            query: vec![("status".into(), "done".into())],
            headers,
            body: br#"{"hello":"world"}"#.to_vec(),
            client_ip: Some("203.0.113.7".into()),
        }
    }

    #[test]
    fn templating_substitutes_params() {
        let body = br#"{"id":"{{id}}","n":"{{name}}"}"#;
        let params = vec![
            ("id".to_string(), "42".to_string()),
            ("name".to_string(), "joe".to_string()),
        ];
        assert_eq!(
            render_body(body, &params, &req()),
            br#"{"id":"42","n":"joe"}"#.to_vec()
        );
    }

    #[test]
    fn templating_request_tokens() {
        let body = b"path={{id}} q={{query.status}} h={{header.x-tenant}} b={{body}}";
        let params = vec![("id".to_string(), "7".to_string())];
        assert_eq!(
            render_body(body, &params, &req()),
            br#"path=7 q=done h=acme b={"hello":"world"}"#.to_vec()
        );
    }

    #[test]
    fn unknown_token_is_empty() {
        assert_eq!(render_body(b"[{{missing}}]", &[], &req()), b"[]".to_vec());
    }
}
