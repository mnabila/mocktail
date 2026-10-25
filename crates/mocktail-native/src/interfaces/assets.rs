//! Embedded SvelteKit portal assets. The static build output
//! is compiled into the binary; the server serves it with SPA fallback.

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../frontend/build"]
struct Assets;

/// Resolve a portal request path to (bytes, content-type), falling back to
/// `index.html` for client-side routes (SPA).
pub fn serve(path: &str) -> Option<(Vec<u8>, String)> {
    // Normalize: "/" and client routes map to index.html.
    let rel = path.trim_start_matches('/');
    let candidate = if rel.is_empty() { "index.html" } else { rel };

    if let Some(file) = Assets::get(candidate) {
        let ct = mime_guess::from_path(candidate)
            .first_or_octet_stream()
            .to_string();
        return Some((file.data.into_owned(), ct));
    }

    // SPA fallback: unknown non-asset path -> index.html (the client router
    // resolves it). Asset-looking paths (with an extension) 404 instead.
    if !candidate.contains('.')
        && let Some(index) = Assets::get("index.html")
    {
        return Some((index.data.into_owned(), "text/html".to_string()));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_is_embedded() {
        let (bytes, ct) = serve("/").expect("index.html embedded in binary");
        assert!(ct.starts_with("text/html"));
        assert!(String::from_utf8_lossy(&bytes).contains("mocktail"));
    }

    #[test]
    fn spa_fallback_for_client_routes() {
        // An unknown extensionless path falls back to the SPA shell.
        assert!(serve("/acme/collections").is_some());
    }

    #[test]
    fn missing_asset_is_none() {
        assert!(serve("/_app/does-not-exist.js").is_none());
    }
}
