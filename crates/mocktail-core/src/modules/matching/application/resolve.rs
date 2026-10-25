//! ResolveMock use case: load a collection's active mocks via the port, run
//! the pure engine, and return an owned result.

use crate::modules::matching::domain::{MatchOutcome, resolve};
use crate::modules::mock::application::MockRepository;
use crate::modules::mock::domain::Mock;
use crate::shared::error::AppResult;
use crate::shared::http::Request;
use crate::shared::ids::{CollectionId, MockId, WorkspaceId};

/// Owned resolution result (decoupled from the borrowed `MatchOutcome` so it
/// can cross an await / be returned from the use case).
pub struct Resolved {
    pub mock: Mock,
    pub params: Vec<(String, String)>,
    pub reason: String,
    pub losers: Vec<MockId>,
}

impl<'a> From<MatchOutcome<'a>> for Resolved {
    fn from(o: MatchOutcome<'a>) -> Self {
        Resolved {
            mock: o.mock.clone(),
            params: o.params,
            reason: o.reason,
            losers: o.losers.into_iter().map(|m| m.id.clone()).collect(),
        }
    }
}

pub async fn resolve_mock(
    repo: &dyn MockRepository,
    ws: &WorkspaceId,
    coll: &CollectionId,
    req: &Request,
    spec_path: &str,
) -> AppResult<Option<Resolved>> {
    let mocks = repo.list_active(ws, coll).await?;
    Ok(resolve(&mocks, req, spec_path).map(Resolved::from))
}
