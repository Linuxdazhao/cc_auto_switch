use crate::AppState;
use axum::{Json, Router, extract::State, routing::get};
use serde_json::{Value, json};

pub fn router() -> Router<AppState> {
    Router::new().route("/api/health", get(health))
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    let request_count = state
        .store
        .list_requests(state.session_id.as_str())
        .await
        .map(|requests| requests.len())
        .unwrap_or(0);
    let store_status = "ok"; // upgraded to "degraded" in Task 21
    Json(json!({
        "status": "ok",
        "provider": state.provider.as_str(),
        "upstream": state.upstream.to_string(),
        "session_id": state.session_id.as_str(),
        "started_at": state.started_at,
        "request_count": request_count,
        "store": store_status,
    }))
}
