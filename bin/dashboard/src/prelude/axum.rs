use axum::http::StatusCode;
use axum::response::IntoResponse;

pub use axum::extract::Path;
pub use axum::extract::State;

/// Wrapper for `anyhow::Error` that implements `IntoResponse`.
pub struct AxumError(anyhow::Error);

impl IntoResponse for AxumError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Internal Server Error: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AxumError
where
    E: Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        Self(value.into())
    }
}
