use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::core::model::{HealthResponse, RefreshResponse, Snapshot};
use crate::core::refresh::{RefreshManager, RefreshStatus};

#[derive(Clone)]
pub struct AppState {
    pub refresh_manager: Arc<RefreshManager>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/refresh", post(refresh))
        .route("/api/refresh-status", get(refresh_status))
        .route("/api/snapshot", get(get_snapshot))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub fn create_router_with_frontend(state: AppState, frontend_path: &str) -> Router {
    let api_router = Router::new()
        .route("/api/health", get(health))
        .route("/api/refresh", post(refresh))
        .route("/api/refresh-status", get(refresh_status))
        .route("/api/snapshot", get(get_snapshot))
        .with_state(state);

    Router::new()
        .merge(api_router)
        .fallback_service(ServeDir::new(frontend_path))
        .layer(CorsLayer::permissive())
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let health = state.refresh_manager.health_check().await;
    Json(health)
}

async fn refresh(
    State(state): State<AppState>,
) -> Result<Json<RefreshStatus>, (StatusCode, String)> {
    match state.refresh_manager.start_refresh().await {
        Ok(status) => Ok(Json(status)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn refresh_status(State(state): State<AppState>) -> Json<RefreshStatus> {
    let status = state.refresh_manager.get_refresh_status().await;
    Json(status)
}

async fn get_snapshot(State(state): State<AppState>) -> Json<Snapshot> {
    let snapshot = state.refresh_manager.get_snapshot().await;
    Json(snapshot)
}
