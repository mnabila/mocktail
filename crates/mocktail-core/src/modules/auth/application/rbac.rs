//! RBAC checks.

use super::ports::MembershipRepository;
use crate::modules::auth::domain::{Membership, Role};
use crate::shared::error::{AppError, AppResult};
use crate::shared::ids::{UserId, WorkspaceId};

/// Require that `user` is a member of `workspace` with at least `required`;
/// returns the membership or a 403.
pub async fn require_role(
    memberships: &dyn MembershipRepository,
    user: &UserId,
    workspace: &WorkspaceId,
    required: Role,
) -> AppResult<Membership> {
    let membership = memberships
        .find(user, workspace)
        .await?
        .ok_or_else(|| AppError::Forbidden("not a member of this workspace".into()))?;
    if !membership.role.allows(required) {
        return Err(AppError::Forbidden(format!(
            "requires {} role",
            required.as_str()
        )));
    }
    Ok(membership)
}
