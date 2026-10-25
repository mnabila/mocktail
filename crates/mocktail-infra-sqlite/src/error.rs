use mocktail_core::shared::error::AppError;

pub fn infra<E: std::fmt::Display>(e: E) -> AppError {
    AppError::Internal(e.to_string())
}
