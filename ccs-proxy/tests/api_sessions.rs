use axum::body::to_bytes;
use ccs_proxy::api::build_api_app;
use ccs_proxy::capture::{CaptureRecord, RequestPart};
use ccs_proxy::store::{FsStore, SessionMeta, Store};
use ccs_proxy::{AppState, ProviderKind, SessionId};
use chrono::Utc;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use tempfile::tempdir;
use tower::ServiceExt;
use url::Url;

#[tokio::test]
async fn lists_sessions_and_requests() {
    let dir = tempdir().unwrap();
    let store = Arc::new(FsStore::open(dir.path().to_path_buf()).unwrap());
    let sid = "s1".to_string();
    store
        .init_session(SessionMeta {
            session_id: sid.clone(),
            provider: "claude".into(),
            upstream: "https://api.anthropic.com".into(),
            proxy_port: 1,
            api_port: 2,
            started_at: Utc::now(),
            ended_at: None,
            request_count: 0,
            schema_version: 1,
            cwd: None,
            models: vec![],
        })
        .await
        .unwrap();
    store
        .append(CaptureRecord {
            seq: 1,
            session_id: sid.clone(),
            request_id: Some("req_1".into()),
            started_at: Utc::now(),
            ended_at: Some(Utc::now()),
            duration_ms: Some(10),
            ttft_ms: Some(1),
            request: RequestPart {
                method: "POST".into(),
                path: "/v1/messages".into(),
                headers: BTreeMap::new(),
                body: json!({"model":"claude"}),
            },
            response: None,
            usage: None,
            model: Some("claude".into()),
            error: None,
            partial: false,
            schema_version: 1,
        })
        .await
        .unwrap();

    let state = AppState::new(
        store.clone(),
        ProviderKind::Claude,
        Url::parse("https://api.anthropic.com").unwrap(),
        SessionId::new(),
        true,
    );
    let app = build_api_app(state);

    // /api/sessions
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/sessions")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        body.as_array()
            .unwrap()
            .iter()
            .any(|session| session["session_id"] == "s1")
    );

    // /api/sessions/s1
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/sessions/s1")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["meta"]["session_id"], "s1");
    assert_eq!(body["requests"].as_array().unwrap().len(), 1);

    // /api/requests/s1/1
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/requests/s1/1")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["seq"], 1);
    assert_eq!(body["request_id"], "req_1");
}
