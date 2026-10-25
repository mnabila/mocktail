//! Final response conversion for the admin and public error envelopes.

use crate::shared::extract::to_axum;
use axum::response::Response as AxumResponse;
use mocktail_core::shared::error::AppResult;
use mocktail_core::shared::http::Response;

pub(crate) fn finish_admin(result: AppResult<Response>) -> AxumResponse {
    match result {
        Ok(resp) => to_axum(resp),
        Err(e) => to_axum(e.to_admin_response()),
    }
}

pub(crate) fn finish_public(result: AppResult<Response>) -> AxumResponse {
    match result {
        Ok(resp) => to_axum(resp),
        Err(e) => to_axum(e.to_response()),
    }
}
