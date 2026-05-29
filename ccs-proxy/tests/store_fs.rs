use ccs_proxy::capture::{CaptureRecord, RequestPart};
use ccs_proxy::store::{FsStore, SessionMeta, Store};
use chrono::Utc;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use tempfile::tempdir;

fn rec(seq: u64, sid: &str) -> CaptureRecord {
    CaptureRecord {
        seq,
        session_id: sid.into(),
        request_id: Some(format!("req_{seq}")),
        started_at: Utc::now(),
        ended_at: Some(Utc::now()),
        duration_ms: Some(1),
        ttft_ms: Some(1),
        request: RequestPart {
            method: "POST".into(),
            path: "/v1/messages".into(),
            headers: BTreeMap::new(),
            body: json!({}),
        },
        response: None,
        usage: None,
        model: Some("claude".into()),
        error: None,
        partial: false,
        schema_version: 1,
    }
}

#[tokio::test]
async fn writes_and_reads_back_records() {
    let dir = tempdir().unwrap();
    let meta = SessionMeta {
        session_id: "s1".into(),
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
    };
    let store: Arc<dyn Store> = Arc::new(FsStore::open(dir.path().to_path_buf()).unwrap());
    store.init_session(meta.clone()).await.unwrap();
    store.append(rec(1, "s1")).await.unwrap();
    store.append(rec(2, "s1")).await.unwrap();

    let sessions = store.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].request_count, 2);

    let listed = store.list_requests("s1").await.unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].seq, 1);

    let got = store.get_request("s1", 2).await.unwrap().unwrap();
    assert_eq!(got.seq, 2);
    assert_eq!(got.request_id.as_deref(), Some("req_2"));
}

#[tokio::test]
async fn missing_session_returns_none() {
    let dir = tempdir().unwrap();
    let store: Arc<dyn Store> = Arc::new(FsStore::open(dir.path().to_path_buf()).unwrap());
    assert!(store.get_request("nope", 1).await.unwrap().is_none());
}

fn rec_with_system(seq: u64, sid: &str, model: &str, system_text: &str) -> CaptureRecord {
    let mut r = rec(seq, sid);
    r.model = Some(model.into());
    r.request.body = json!({
        "system": system_text,
        "model": model,
    });
    r
}

#[tokio::test]
async fn append_backfills_cwd_from_system_prompt() {
    let dir = tempdir().unwrap();
    let meta = SessionMeta {
        session_id: "s_cwd".into(),
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
    };
    let store: Arc<dyn Store> = Arc::new(FsStore::open(dir.path().to_path_buf()).unwrap());
    store.init_session(meta).await.unwrap();
    store
        .append(rec_with_system(
            1,
            "s_cwd",
            "claude-opus-4-7",
            "Primary working directory: /Users/me/proj-a\nrest",
        ))
        .await
        .unwrap();

    let sessions = store.list_sessions().await.unwrap();
    assert_eq!(sessions[0].cwd.as_deref(), Some("/Users/me/proj-a"));
    assert_eq!(sessions[0].models, vec!["claude-opus-4-7".to_string()]);
}

#[tokio::test]
async fn append_does_not_overwrite_existing_cwd() {
    let dir = tempdir().unwrap();
    let meta = SessionMeta {
        session_id: "s_cwd2".into(),
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
    };
    let store: Arc<dyn Store> = Arc::new(FsStore::open(dir.path().to_path_buf()).unwrap());
    store.init_session(meta).await.unwrap();
    store
        .append(rec_with_system(
            1, "s_cwd2", "claude-opus-4-7",
            "Primary working directory: /first\n",
        ))
        .await
        .unwrap();
    // Second request with a different cwd marker — should be ignored.
    store
        .append(rec_with_system(
            2, "s_cwd2", "claude-opus-4-7",
            "Primary working directory: /second\n",
        ))
        .await
        .unwrap();

    let sessions = store.list_sessions().await.unwrap();
    assert_eq!(sessions[0].cwd.as_deref(), Some("/first"));
}

#[tokio::test]
async fn append_dedupes_and_appends_models() {
    let dir = tempdir().unwrap();
    let meta = SessionMeta {
        session_id: "s_m".into(),
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
    };
    let store: Arc<dyn Store> = Arc::new(FsStore::open(dir.path().to_path_buf()).unwrap());
    store.init_session(meta).await.unwrap();
    store.append(rec_with_system(1, "s_m", "claude-opus-4-7", "")).await.unwrap();
    store.append(rec_with_system(2, "s_m", "claude-sonnet-4-6", "")).await.unwrap();
    store.append(rec_with_system(3, "s_m", "claude-opus-4-7", "")).await.unwrap();

    let sessions = store.list_sessions().await.unwrap();
    assert_eq!(
        sessions[0].models,
        vec!["claude-opus-4-7".to_string(), "claude-sonnet-4-6".to_string()]
    );
}

#[tokio::test]
async fn list_sessions_reads_old_meta_without_cwd_or_models() {
    let dir = tempdir().unwrap();
    let store: Arc<dyn Store> = Arc::new(FsStore::open(dir.path().to_path_buf()).unwrap());

    // Hand-write a meta.json missing cwd + models fields (old schema).
    let sid = "legacy_session";
    let session_dir = dir.path().join("sessions").join(sid);
    std::fs::create_dir_all(&session_dir).unwrap();
    let legacy = serde_json::json!({
        "session_id": sid,
        "provider": "claude",
        "upstream": "https://api.anthropic.com",
        "proxy_port": 1,
        "api_port": 2,
        "started_at": "2026-05-01T00:00:00Z",
        "ended_at": null,
        "request_count": 0,
        "schema_version": 1,
    });
    std::fs::write(
        session_dir.join("meta.json"),
        serde_json::to_vec_pretty(&legacy).unwrap(),
    )
    .unwrap();

    let sessions = store.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, sid);
    assert_eq!(sessions[0].cwd, None);
    assert!(sessions[0].models.is_empty());
}
