//! Path template matching.

/// Split a path into segments, ignoring leading/trailing slashes.
/// `"/"` and `""` both yield an empty segment list (the root).
fn segments(path: &str) -> Vec<&str> {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        Vec::new()
    } else {
        trimmed.split('/').collect()
    }
}

fn is_param(seg: &str) -> bool {
    seg.len() >= 2 && seg.starts_with('{') && seg.ends_with('}')
}

fn param_name(seg: &str) -> &str {
    &seg[1..seg.len() - 1]
}

/// Match a path template against a concrete path. Returns captured params on a
/// match (literal segments must be equal; `{name}` captures one segment).
pub fn match_path(template: &str, path: &str) -> Option<Vec<(String, String)>> {
    let tsegs = segments(template);
    let psegs = segments(path);
    if tsegs.len() != psegs.len() {
        return None;
    }
    let mut params = Vec::new();
    for (t, p) in tsegs.iter().zip(psegs.iter()) {
        if is_param(t) {
            params.push((param_name(t).to_string(), (*p).to_string()));
        } else if t != p {
            return None;
        }
    }
    Some(params)
}

/// Count literal (non-param) segments — the specificity measure.
pub(super) fn literal_count(template: &str) -> usize {
    segments(template).iter().filter(|s| !is_param(s)).count()
}
