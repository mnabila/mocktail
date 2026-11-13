//! Workspace aggregate. A workspace is the ownership
//! boundary and resolves as a separator-delimited host on the mock surface.

use crate::shared::ids::{Slug, WorkspaceId};
use crate::shared::ports::UnixMillis;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub slug: Slug,
    pub created_at: UnixMillis,
}
