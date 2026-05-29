//! Regression test: JSON body keys flagged by `capture::redact::redact_body`
//! must arrive on disk replaced with `"<redacted>"`, while non-secret keys
//! (e.g. `model`) survive untouched. This locks in the redact wiring in the
//! capture pipeline (Task 14) and the default-on `redact=true` path.

use ccs_proxy::proxy::build_proxy_app;
use ccs_proxy::store::{FsStore, SessionMeta, Store};
use ccs_proxy::{AppState, ProviderKind, SessionId};
use chrono::Utc;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;
use url::Url;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn redacts_token_in_request_body() {
    let dir = tempdir().unwrap();
    let mock = MockServer::start().await;
    Mock::given(any())
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(
                    "event: message_start\n\
                     data: {\"type\":\"message_start\",\"message\":{\"id\":\"x\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"m\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\
                     \n\
                     event: message_stop\n\
                     data: {\"type\":\"message_stop\"}\n\
                     \n",
                ),
        )
        .mount(&mock)
        .await;

    let store: Arc<dyn Store> = Arc::new(FsStore::open(dir.path().to_path_buf()).unwrap());
    let sid = SessionId::new();
    store
        .init_session(SessionMeta {
            session_id: sid.to_string(),
            provider: "claude".into(),
            upstream: mock.uri(),
            proxy_port: 0,
            api_port: 0,
            started_at: Utc::now(),
            ended_at: None,
            request_count: 0,
            schema_version: 1,
            cwd: None,
            models: vec![],
        })
        .await
        .unwrap();

    let state = AppState::new(
        store.clone(),
        ProviderKind::Claude,
        Url::parse(&mock.uri()).unwrap(),
        sid.clone(),
        true,
    );
    let app = build_proxy_app(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    reqwest::Client::new()
        .post(format!("http://{addr}/v1/messages"))
        .header("content-type", "application/json")
        .body(r#"{"model":"m","api_key":"sk-leak","messages":[]}"#)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let rec = store
        .get_request(sid.as_str(), 1)
        .await
        .unwrap()
        .expect("record at seq=1 should exist");
    assert_eq!(rec.request.body["api_key"], "<redacted>");
    assert_eq!(rec.request.body["model"], "m");
}
