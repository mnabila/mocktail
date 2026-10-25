//! Request cookie extraction.

use crate::shared::http::Request;

/// Extract a cookie value from the request `Cookie` header.
pub fn cookie(req: &Request, name: &str) -> Option<String> {
    let header = req.headers.get("cookie")?;
    for part in header.split(';') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=')
            && k.trim() == name
        {
            return Some(v.trim().to_string());
        }
    }
    None
}
