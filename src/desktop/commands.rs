use std::sync::Arc;

use crate::core::model::{HealthResponse, RefreshResponse, Snapshot};
use crate::core::refresh::RefreshManager;

pub struct DesktopState {
    pub refresh_manager: Arc<RefreshManager>,
}

#[tauri::command]
pub async fn health(state: tauri::State<'_, DesktopState>) -> Result<HealthResponse, String> {
    Ok(state.refresh_manager.health_check().await)
}

#[tauri::command]
pub async fn refresh(state: tauri::State<'_, DesktopState>) -> Result<RefreshResponse, String> {
    state
        .refresh_manager
        .refresh()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_snapshot(state: tauri::State<'_, DesktopState>) -> Result<Snapshot, String> {
    Ok(state.refresh_manager.get_snapshot().await)
}
