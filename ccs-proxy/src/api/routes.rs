use crate::AppState;
use axum::response::IntoResponse;
use axum::{Json, Router, extract::State, routing::get};
use serde_json::{Value, json};

type RequestPathParams = axum::extract::Path<(String, u64)>;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions/{sid}", get(get_session))
        .route("/api/requests/{sid}/{seq}", get(get_request))
        .route("/api/stream", get(crate::api::stream::stream))
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    let request_count = state
        .store
        .list_requests(state.session_id.as_str())
        .await
        .map(|requests| requests.len())
        .unwrap_or(0);
    let store_status = if let Some(fs) = state
        .store
        .as_any()
        .downcast_ref::<crate::store::FsStore>()
    {
        if fs.consecutive_write_failures() >= 10 {
            "degraded"
        } else {
            "ok"
        }
    } else {
        "ok"
    };
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

async fn list_sessions(State(state): State<AppState>) -> Json<Value> {
    let metas = state.store.list_sessions().await.unwrap_or_default();
    Json(serde_json::to_value(metas).unwrap_or(Value::Null))
}

async fn get_session(
    State(state): State<AppState>,
    axum::extract::Path(sid): axum::extract::Path<String>,
) -> axum::response::Response {
    let metas = state.store.list_sessions().await.unwrap_or_default();
    let meta = metas.into_iter().find(|item| item.session_id == sid);
    let requests = state.store.list_requests(&sid).await.unwrap_or_default();
    match meta {
        Some(found) => Json(json!({"meta": found, "requests": requests})).into_response(),
        None => (axum::http::StatusCode::NOT_FOUND, "session not found").into_response(),
    }
}

async fn get_request(
    State(state): State<AppState>,
    axum::extract::Path((sid, seq)): RequestPathParams,
) -> axum::response::Response {
    match state.store.get_request(&sid, seq).await {
        Ok(Some(rec)) => match serde_json::to_value(&rec) {
            Ok(value) => Json(value).into_response(),
            Err(err) => {
                tracing::warn!(?err, "failed to serialize capture record");
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "store error").into_response()
            }
        },
        Ok(None) => (axum::http::StatusCode::NOT_FOUND, "request not found").into_response(),
        Err(err) => {
            tracing::warn!(?err, "store error");
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "store error").into_response()
        }
    }
}
