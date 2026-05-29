#[cfg(unix)]
mod daemon_aggregate {
    use ccs_proxy::capture::{CaptureRecord, RequestPart, ResponsePart, Usage};
    use ccs_proxy::store::{FsStore, SessionMeta, Store};
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[tokio::test]
    async fn aggregate_health_returns_proxy_count() {
        let tmp_a = TempDir::new().unwrap();
        let tmp_b = TempDir::new().unwrap();
        let store_a = Arc::new(FsStore::open(tmp_a.path().to_path_buf()).unwrap());
        let store_b = Arc::new(FsStore::open(tmp_b.path().to_path_buf()).unwrap());

        let stores = vec![
            ("https://a.example.com".to_string(), store_a),
            ("https://b.example.com".to_string(), store_b),
        ];

        let alias_map = Arc::new(cc_switch::daemon::aggregate::state::AliasMap::from_entries(
            vec![(
                "https://a.example.com".to_string(),
                vec!["alias_a".to_string()],
            )],
        ));

        let handle = cc_switch::daemon::aggregate::serve(stores, vec![], alias_map, 0)
            .await
            .unwrap();

        let resp = reqwest::get(format!("http://127.0.0.1:{}/api/health", handle.port))
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["proxy_count"], 2);
        assert_eq!(body["status"], "ok");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn aggregate_sessions_merges_across_stores() {
        let tmp_a = TempDir::new().unwrap();
        let tmp_b = TempDir::new().unwrap();
        let store_a = Arc::new(FsStore::open(tmp_a.path().to_path_buf()).unwrap());
        let store_b = Arc::new(FsStore::open(tmp_b.path().to_path_buf()).unwrap());

        store_a
            .init_session(SessionMeta {
                session_id: "sess_a".to_string(),
                provider: "claude".to_string(),
                upstream: "https://a.example.com".to_string(),
                proxy_port: 0,
                api_port: 0,
                started_at: chrono::Utc::now(),
                ended_at: None,
                request_count: 1,
                schema_version: 1,
                cwd: None,
                models: vec![],
            })
            .await
            .unwrap();

        store_b
            .init_session(SessionMeta {
                session_id: "sess_b".to_string(),
                provider: "claude".to_string(),
                upstream: "https://b.example.com".to_string(),
                proxy_port: 0,
                api_port: 0,
                started_at: chrono::Utc::now() - chrono::Duration::hours(1),
                ended_at: None,
                request_count: 2,
                schema_version: 1,
                cwd: None,
                models: vec![],
            })
            .await
            .unwrap();

        let stores = vec![
            ("https://a.example.com".to_string(), store_a),
            ("https://b.example.com".to_string(), store_b),
        ];

        let alias_map = Arc::new(cc_switch::daemon::aggregate::state::AliasMap::from_entries(
            vec![
                (
                    "https://a.example.com".to_string(),
                    vec!["work".to_string()],
                ),
                (
                    "https://b.example.com".to_string(),
                    vec!["personal".to_string()],
                ),
            ],
        ));

        let handle = cc_switch::daemon::aggregate::serve(stores, vec![], alias_map, 0)
            .await
            .unwrap();

        // Test /api/sessions returns both sessions
        let resp = reqwest::get(format!("http://127.0.0.1:{}/api/sessions", handle.port))
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0]["session_id"], "sess_a");
        assert_eq!(sessions[0]["upstream"], "https://a.example.com");
        assert_eq!(sessions[0]["aliases"][0], "work");

        // Test filter by upstream
        let resp = reqwest::get(format!(
            "http://127.0.0.1:{}/api/sessions?upstream=https://b.example.com",
            handle.port
        ))
        .await
        .unwrap();
        let sessions: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["session_id"], "sess_b");

        // Test filter by alias
        let resp = reqwest::get(format!(
            "http://127.0.0.1:{}/api/sessions?alias=work",
            handle.port
        ))
        .await
        .unwrap();
        let sessions: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["session_id"], "sess_a");

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn aggregate_stats_computes_per_upstream() {
        let tmp_a = TempDir::new().unwrap();
        let store_a = Arc::new(FsStore::open(tmp_a.path().to_path_buf()).unwrap());

        store_a
            .init_session(SessionMeta {
                session_id: "sess1".to_string(),
                provider: "claude".to_string(),
                upstream: "https://a.example.com".to_string(),
                proxy_port: 0,
                api_port: 0,
                started_at: chrono::Utc::now(),
                ended_at: None,
                request_count: 0,
                schema_version: 1,
                cwd: None,
                models: vec![],
            })
            .await
            .unwrap();

        store_a
            .append(CaptureRecord {
                seq: 1,
                session_id: "sess1".to_string(),
                request_id: Some("req_1".to_string()),
                started_at: chrono::Utc::now(),
                ended_at: Some(chrono::Utc::now()),
                duration_ms: Some(1000),
                ttft_ms: Some(200),
                request: RequestPart {
                    method: "POST".into(),
                    path: "/v1/messages".into(),
                    headers: BTreeMap::new(),
                    body: serde_json::Value::Null,
                },
                response: Some(ResponsePart {
                    status: 200,
                    headers: BTreeMap::new(),
                    body_reassembled: None,
                    raw_sse_text: None,
                    raw_sse_frames_count: 0,
                }),
                usage: Some(Usage {
                    input_tokens: 100,
                    output_tokens: 50,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                }),
                model: Some("claude-sonnet-4-6".to_string()),
                error: None,
                partial: false,
                schema_version: 1,
            })
            .await
            .unwrap();

        let stores = vec![("https://a.example.com".to_string(), store_a)];
        let alias_map = Arc::new(cc_switch::daemon::aggregate::state::AliasMap::from_entries(
            vec![],
        ));
        let handle = cc_switch::daemon::aggregate::serve(stores, vec![], alias_map, 0)
            .await
            .unwrap();

        let resp = reqwest::get(format!("http://127.0.0.1:{}/api/stats", handle.port))
            .await
            .unwrap();
        let stats: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(stats["upstreams"][0]["total_requests"], 1);
        assert_eq!(stats["upstreams"][0]["total_input_tokens"], 100);
        assert_eq!(stats["totals"]["total_requests"], 1);

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn aggregate_serves_dashboard_html() {
        let tmp = TempDir::new().unwrap();
        let store = Arc::new(FsStore::open(tmp.path().to_path_buf()).unwrap());

        let stores = vec![("https://a.example.com".to_string(), store)];
        let alias_map = Arc::new(cc_switch::daemon::aggregate::state::AliasMap::from_entries(
            vec![],
        ));
        let handle = cc_switch::daemon::aggregate::serve(stores, vec![], alias_map, 0)
            .await
            .unwrap();

        let resp = reqwest::get(format!("http://127.0.0.1:{}/", handle.port))
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let ct = resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(ct.contains("text/html"));
        let body = resp.text().await.unwrap();
        assert!(body.contains("ccs-daemon"));

        handle.shutdown().await;
    }
}
