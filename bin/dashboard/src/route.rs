use crate::api;
use crate::middlewares::authorized_token_opt;
use crate::state::AppState;
use axum::middleware::map_request_with_state;
use axum::routing;
use axum::Router;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub fn make(state: Arc<AppState>) -> Router {
    Router::new()
        .nest("/api/auth", make_auth(state.clone()))
        .nest("/api/agent", make_agent(state.clone()))
        .nest("/api/admin", make_admin(state.clone()))
        .nest("/api/dashboard", make_dashboard(state.clone()))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

fn make_auth(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/init", routing::post(|| async { "" }))
        .route("/captcha", routing::get(|| async { "" }))
        .route("/authorize", routing::get(|| async { "" }))
        .route("/authorize", routing::post(|| async { "" }))
        .layer(map_request_with_state(state.clone(), authorized_token_opt))
}

fn make_agent(_: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/{machine_id}/config", routing::get(api::agent::config))
        .route("/{machine_id}/report", routing::post(api::agent::report))
        .route("/{machine_id}/report", routing::get(api::agent::websocket))
}

fn make_admin(_: Arc<AppState>) -> Router<Arc<AppState>> {
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

fn make_dashboard(_: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/config", routing::get(|| async { "" }))
        .route("/summary", routing::get(|| async { "" }))
        .route("/hosts", routing::get(|| async { "" }))
        .route("/hosts/{id}", routing::get(|| async { "" }))
}
