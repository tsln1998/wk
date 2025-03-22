use crate::state::AppState;
use axum::extract::Request;
use axum::extract::State;
use axum::http::StatusCode;
use jsonwebtoken::Validation;
use sea_orm::prelude::Uuid;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;

/// Authorization header constants
const AUTHORIZATION_HEADER: &str = "Authorization";
const AUTHORIZATION_PREFIX: &str = "Bearer";

/// Represents an authorized token.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthorizedToken {
    pub uid: Uuid,
    pub nbf: usize,
    pub exp: usize,
}

/// Extracts the authorized token from the request and stores it in the request's extensions.
///
/// If the token does not exist, it will be resolved using the `resolve_token` function.
///
/// If the token can be resolved, it will be stored in the request's extensions under the key
/// `AuthorizedToken`. If not, the request will be passed to the next handler without any modifications.
///
/// # Errors
///
/// Returns `StatusCode::UNAUTHORIZED` if the token does not exist or cannot be resolved.
///
#[allow(dead_code)]
pub async fn authorized_token<B>(
    State(state): State<Arc<AppState>>,
    mut req: Request<B>,
) -> Result<Request<B>, StatusCode> {
    let token = resolve_token(&state, &req)?;
    req.extensions_mut().insert(token.clone());
    req.extensions_mut().insert(Some(token));

    Ok(req)
}

/// Extracts the authorized token from the request and stores it in the request's extensions if it exists.
///
/// If the token does not exist, the request will be passed to the next handler without any modifications.
#[allow(dead_code)]
pub async fn authorized_token_opt<B>(
    State(state): State<Arc<AppState>>,
    mut req: Request<B>,
) -> Result<Request<B>, StatusCode> {
    if let Ok(token) = resolve_token(&state, &req) {
        req.extensions_mut().insert(Some(token));
    }

    Ok(req)
}

/// Resolves the authorized token from the request.
///
/// This function extracts the token from the `Authorization` header and decodes it using the JWT
/// configuration in the app state. If the token does not exist or cannot be resolved, it returns
/// `StatusCode::UNAUTHORIZED`.
///
/// # Errors
///
/// Returns `StatusCode::UNAUTHORIZED` if the token does not exist or cannot be resolved.
///
/// Returns `StatusCode::INTERNAL_SERVER_ERROR` if the app state is not present in the request's
/// extensions.
fn resolve_token<B>(state: &AppState, req: &Request<B>) -> Result<AuthorizedToken, StatusCode> {
    // get token from request
    let token = req
        .headers()
        .get(AUTHORIZATION_HEADER)
        .and_then(|header| header.to_str().ok())
        .and_then(|value| value.strip_prefix(AUTHORIZATION_PREFIX))
        .map(|token| token.trim_start())
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_owned();

    // decode token using jwt
    let decoded = jsonwebtoken::decode::<AuthorizedToken>(
        &token,
        &state.jwt.decoding,
        &Validation::default(),
    )
    .map(|v| v.claims)
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(decoded)
}
