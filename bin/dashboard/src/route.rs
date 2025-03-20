use crate::api;
use crate::state::AppState;
use axum::routing;
use axum::Router;

pub fn make() -> Router<AppState> {
    Router::new()
        .nest("/api/auth", make_auth())
        .nest("/api/agent", make_agent())
        .nest("/api/admin", make_admin())
        .nest("/api/dashboard", make_dashboard())
}

pub fn make_auth() -> Router<AppState> {
    Router::new()
        .route("/init", routing::post(|| async { "" }))
        .route("/captcha", routing::get(|| async { "" }))
        .route("/authorize", routing::get(|| async { "" }))
        .route("/authorize", routing::post(|| async { "" }))
}

pub fn make_agent() -> Router<AppState> {
    Router::new()
        .route("/config", routing::get(api::agent::config))
        .route("/report", routing::any(|| async { "" }))
}

pub fn make_admin() -> Router<AppState> {
    Router::new()
        .route("/config", routing::get(|| async { "" }))
        .route("/config", routing::post(|| async { "" }))
        .route("/hosts", routing::get(|| async { "" }))
        .route("/hosts", routing::post(|| async { "" }))
        .route("/hosts/{id}", routing::get(|| async { "" }))
        .route("/hosts/{id}", routing::put(|| async { "" }))
        .route("/hosts/{id}", routing::delete(|| async { "" }))
        .route("/users", routing::get(|| async { "" }))
        .route("/users", routing::post(|| async { "" }))
        .route("/users/{id}", routing::get(|| async { "" }))
        .route("/users/{id}", routing::put(|| async { "" }))
        .route("/users/{id}", routing::delete(|| async { "" }))
}

pub fn make_dashboard() -> Router<AppState> {
    Router::new()
        .route("/config", routing::get(|| async { "" }))
        .route("/summary", routing::get(|| async { "" }))
        .route("/hosts", routing::get(|| async { "" }))
        .route("/hosts/{id}", routing::get(|| async { "" }))
}
