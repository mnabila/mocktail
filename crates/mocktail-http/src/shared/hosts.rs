//! Host-first routing helpers.

/// Strip a `:port` suffix and lowercase a host header value.
pub fn host_without_port(host: &str) -> String {
    let h = host.split(':').next().unwrap_or(host);
    h.trim().to_ascii_lowercase()
}

/// Classify a host against the apex domain.
pub enum Surface<'a> {
    /// Admin surface on the apex domain.
    Admin,
    /// Mock runtime on `{workspace-slug}<separator>apex`.
    Workspace(&'a str),
    /// Host does not belong to this deployment.
    Unknown,
}

pub fn classify<'a>(host: &'a str, apex: &str, separators: &str) -> Surface<'a> {
    if host == apex || host == "localhost" {
        return Surface::Admin;
    }
    let label = host
        .strip_suffix(apex)
        .and_then(|rest| rest.strip_suffix(|c: char| separators.contains(c)));
    // Only a single leading label is a workspace (no nested subdomains).
    match label {
        Some(label) if !label.is_empty() && !label.contains('.') => Surface::Workspace(label),
        _ => Surface::Unknown,
    }
}

/// The first path segment (the collection slug on the mock surface).
pub fn first_segment(path: &str) -> &str {
    path.trim_start_matches('/').split('/').next().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_host() {
        let both = ".-";
        assert!(matches!(
            classify("mocktail.com", "mocktail.com", both),
            Surface::Admin
        ));
        assert!(matches!(
            classify("acme.mocktail.com", "mocktail.com", both),
            Surface::Workspace("acme")
        ));
        assert!(matches!(
            classify("acme-mocktail.com", "mocktail.com", both),
            Surface::Workspace("acme")
        ));
        assert!(matches!(
            classify("acme-team-mocktail.com", "mocktail.com", both),
            Surface::Workspace("acme-team")
        ));
        assert!(matches!(
            classify("a.b.mocktail.com", "mocktail.com", both),
            Surface::Unknown
        ));
        assert!(matches!(
            classify("a.b-mocktail.com", "mocktail.com", both),
            Surface::Unknown
        ));
        assert!(matches!(
            classify("-mocktail.com", "mocktail.com", both),
            Surface::Unknown
        ));
        assert!(matches!(
            classify("evil.com", "mocktail.com", both),
            Surface::Unknown
        ));
        assert!(matches!(
            classify("acme-localhost", "localhost", both),
            Surface::Workspace("acme")
        ));
    }

    #[test]
    fn separators_gate_forms() {
        assert!(matches!(
            classify("acme.mocktail.com", "mocktail.com", "."),
            Surface::Workspace("acme")
        ));
        assert!(matches!(
            classify("acme-mocktail.com", "mocktail.com", "."),
            Surface::Unknown
        ));
        assert!(matches!(
            classify("acme-mocktail.com", "mocktail.com", "-"),
            Surface::Workspace("acme")
        ));
        assert!(matches!(
            classify("acme.mocktail.com", "mocktail.com", "-"),
            Surface::Unknown
        ));
    }

    #[test]
    fn port_stripped() {
        assert_eq!(
            host_without_port("acme.mocktail.com:8787"),
            "acme.mocktail.com"
        );
    }

    #[test]
    fn first_seg() {
        assert_eq!(first_segment("/todo/api/v1/x"), "todo");
        assert_eq!(first_segment("/"), "");
    }
}
