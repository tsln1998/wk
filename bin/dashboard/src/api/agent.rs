use crate::state::AppState;
use axum::extract::Query;
use axum::extract::State;
use database::models::prelude::*;
use sea_orm::EntityTrait;

pub async fn config(State(state): State<AppState>, Query(machine_id): Query<String>) {}
