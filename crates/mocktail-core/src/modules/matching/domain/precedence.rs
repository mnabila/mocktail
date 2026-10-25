//! Precedence selection and diagnostics.

use super::matcher::mock_matches;
use super::path::literal_count;
use crate::modules::mock::domain::Mock;
use crate::shared::http::Request;

/// The precedence sort key. Higher tuple wins, compared
/// lexicographically: priority, then specificity (literal segments, then
/// matcher count), then most-recently-updated.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct PrecedenceKey {
    priority: i32,
    literal_segments: usize,
    matcher_count: usize,
    updated_at: i64,
}

fn key_for(mock: &Mock) -> PrecedenceKey {
    PrecedenceKey {
        priority: mock.priority.0,
        literal_segments: literal_count(&mock.path_template),
        matcher_count: mock.matchers.len(),
        updated_at: mock.updated_at,
    }
}

pub struct MatchOutcome<'a> {
    pub mock: &'a Mock,
    pub params: Vec<(String, String)>,
    /// Human-readable precedence reason (logged).
    pub reason: String,
    pub losers: Vec<&'a Mock>,
}

/// Resolve the winning mock for a request against a mount-stripped `spec_path`.
pub fn resolve<'a>(mocks: &'a [Mock], req: &Request, spec_path: &str) -> Option<MatchOutcome<'a>> {
    // Gather matching candidates with their captured params.
    let mut candidates: Vec<(&'a Mock, Vec<(String, String)>)> = mocks
        .iter()
        .filter_map(|m| mock_matches(m, req, spec_path).map(|p| (m, p)))
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // Pick the max by precedence key; stable so ties keep input order before
    // the final updated_at tiebreak inside the key.
    candidates.sort_by_key(|c| std::cmp::Reverse(key_for(c.0)));

    let (winner, params) = candidates[0].clone();
    let reason = if candidates.len() == 1 {
        "only matching mock".to_string()
    } else {
        decide_reason(winner, candidates[1].0)
    };
    let losers = candidates[1..].iter().map(|(m, _)| *m).collect();

    Some(MatchOutcome {
        mock: winner,
        params,
        reason,
        losers,
    })
}

/// Explain why `winner` beat the runner-up, walking the rules in order.
fn decide_reason(winner: &Mock, runner_up: &Mock) -> String {
    let w = key_for(winner);
    let r = key_for(runner_up);
    if w.priority != r.priority {
        return format!("higher priority ({} > {})", w.priority, r.priority);
    }
    if w.literal_segments != r.literal_segments {
        return format!(
            "more specific path ({} literal segments > {})",
            w.literal_segments, r.literal_segments
        );
    }
    if w.matcher_count != r.matcher_count {
        return format!("more matchers ({} > {})", w.matcher_count, r.matcher_count);
    }
    "most recently updated".to_string()
}
