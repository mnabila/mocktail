//! Login and explicit account-linking use cases.

use super::oauth::CallbackOutcome;
use super::ports::UserRepository;
use crate::modules::auth::domain::User;
use crate::shared::error::{AppError, AppResult};
use crate::shared::ids::UserId;
use crate::shared::ports::{Clock, IdGenerator};

/// Sign in (or create) a user from a login callback. Lookup is strictly by
/// `(provider, subject)` — never by email.
pub async fn complete_login(
    users: &dyn UserRepository,
    ids: &dyn IdGenerator,
    clock: &dyn Clock,
    outcome: &CallbackOutcome,
) -> AppResult<(User, bool)> {
    if let Some(existing) = users
        .find_by_identity(&outcome.provider, &outcome.subject)
        .await?
    {
        return Ok((existing, false));
    }
    let display_name = outcome
        .email
        .clone()
        .unwrap_or_else(|| format!("{}:{}", outcome.provider, outcome.subject));
    let user = User {
        id: UserId::new(ids.new_object_id()),
        username: format!("user_{}", ids.new_object_id()),
        display_name,
        created_at: clock.now_millis(),
    };
    users.insert(&user).await?;
    users.add_identity(
        &user.id,
        &outcome.provider,
        &outcome.subject,
        outcome.email.clone(),
    ).await?;
    Ok((user, true))
}

/// Explicitly link a new identity to the signed-in user. A
/// `(provider, subject)` already linked to another account is rejected.
pub async fn connect_provider(
    users: &dyn UserRepository,
    current_user: &UserId,
    outcome: &CallbackOutcome,
) -> AppResult<User> {
    if let Some(owner) = users
        .find_by_identity(&outcome.provider, &outcome.subject)
        .await?
        && owner.id.as_str() != current_user.as_str()
    {
        return Err(AppError::Conflict(
            "this identity is already linked to another account".into(),
        ));
    }
    let user = users
        .find_by_id(current_user)
        .await?
        .ok_or_else(|| AppError::NotFound("current user".into()))?;

    users.add_identity(
        &user.id,
        &outcome.provider,
        &outcome.subject,
        outcome.email.clone(),
    ).await?;

    Ok(user)
}
