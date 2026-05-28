use axum::body::to_bytes;
use ccs_proxy::api::build_api_app;
use ccs_proxy::store::FsStore;
use ccs_proxy::{AppState, ProviderKind, SessionId};
use std::sync::Arc;
use tempfile::tempdir;
use tower::ServiceExt;
use url::Url;

#[tokio::test]
async fn health_returns_ok() {
    let dir = tempdir().unwrap();
    let store = Arc::new(FsStore::open(dir.path().to_path_buf()).unwrap());
    let state = AppState::new(
        store,
        ProviderKind::Claude,
        Url::parse("https://api.anthropic.com").unwrap(),
        SessionId::new(),
        true,
    );
    let app = build_api_app(state);
    let resp = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/health")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["provider"], "claude");
    assert_eq!(json["store"], "ok");
}

#[tokio::test]
async fn serves_dashboard_html() {
    let dir = tempfile::tempdir().unwrap();
    let store =
        std::sync::Arc::new(ccs_proxy::store::FsStore::open(dir.path().to_path_buf()).unwrap());
    let state = ccs_proxy::AppState::new(
        store,
        ccs_proxy::ProviderKind::Claude,
        url::Url::parse("https://api.anthropic.com").unwrap(),
        ccs_proxy::SessionId::new(),
        true,
    );
    let app = ccs_proxy::api::build_api_app(state);
    let resp = tower::ServiceExt::oneshot(
        app,
        axum::http::Request::builder()
            .uri("/")
            .body(axum::body::Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap();
    let body = String::from_utf8_lossy(&bytes);
    assert!(body.contains("ccs-proxy dashboard"));
}
