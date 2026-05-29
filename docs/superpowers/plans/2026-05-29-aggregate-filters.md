# Aggregate Dashboard Filters Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add model + cwd sidebar filters and a "show empty sessions" toggle to the aggregate dashboard, fixing the "click session → empty table" issue.

**Architecture:** Extend `SessionMeta` with `cwd: Option<String>` and `models: Vec<String>`, populated lazily in `FsStore::append` by parsing `Primary working directory:` out of the request's system prompt. Extend `/api/sessions` with `model` / `cwd` / `include_empty` filter params, add a new `/api/meta` endpoint exposing the global dedup'd lists. Frontend reads `/api/meta`, renders two new sidebar checkbox blocks, and adds a "Show empty sessions" toggle visible in the Sessions view.

**Tech Stack:** Rust (axum, serde, tokio, serde_json), vanilla JS + Vue3 (dashboard).

**Spec:** `docs/superpowers/specs/2026-05-29-aggregate-filters-design.md`

---

### Task 1: cwd extraction utility (pure function + unit tests)

**Files:**
- Modify: `ccs-proxy/src/capture/extract.rs`

- [ ] **Step 1: Write the failing tests**

Append these tests to the existing `#[cfg(test)] mod tests` block in `ccs-proxy/src/capture/extract.rs`:

```rust
    #[test]
    fn cwd_from_string_system() {
        let body = json!({
            "system": "You are Claude Code.\nPrimary working directory: /Users/me/proj\nMore text.",
        });
        assert_eq!(extract_cwd(&body), Some("/Users/me/proj".into()));
    }

    #[test]
    fn cwd_from_block_list_system() {
        let body = json!({
            "system": [
                {"type": "text", "text": "header"},
                {"type": "text", "text": "intro\nPrimary working directory: /tmp/x y z\ntail"},
            ],
        });
        assert_eq!(extract_cwd(&body), Some("/tmp/x y z".into()));
    }

    #[test]
    fn cwd_ignores_prose_mention() {
        let body = json!({
            "system": "Consider the user's working directory when answering questions.",
        });
        assert_eq!(extract_cwd(&body), None);
    }

    #[test]
    fn cwd_returns_none_when_no_system() {
        let body = json!({"messages": []});
        assert_eq!(extract_cwd(&body), None);
    }

    #[test]
    fn cwd_takes_first_match_only() {
        let body = json!({
            "system": "Primary working directory: /a\nPrimary working directory: /b",
        });
        assert_eq!(extract_cwd(&body), Some("/a".into()));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p ccs-proxy capture::extract::tests::cwd_ -- --nocapture`
Expected: compile error — `extract_cwd` not defined.

- [ ] **Step 3: Implement `extract_cwd`**

Add this function above the `#[cfg(test)]` block in `ccs-proxy/src/capture/extract.rs`:

```rust
/// Extract the Claude Code working-directory marker from a request body's
/// `system` field. Looks for a line starting with `Primary working directory:`
/// (case-sensitive, anchored to line start) and returns the trimmed path.
/// Handles both `system: "..."` (string) and
/// `system: [{"type":"text","text":"..."}]` (block list) shapes.
pub fn extract_cwd(body: &Value) -> Option<String> {
    let system = body.get("system")?;
    if let Some(s) = system.as_str() {
        return scan_system_text(s);
    }
    if let Some(arr) = system.as_array() {
        for block in arr {
            if let Some(text) = block.get("text").and_then(|v| v.as_str())
                && let Some(found) = scan_system_text(text)
            {
                return Some(found);
            }
        }
    }
    None
}

fn scan_system_text(text: &str) -> Option<String> {
    const MARKER: &str = "Primary working directory:";
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix(MARKER) {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p ccs-proxy capture::extract::tests -- --nocapture`
Expected: all 7 tests pass (2 existing request-id, 2 existing model, plus 5 new cwd).

- [ ] **Step 5: Commit**

```bash
git add ccs-proxy/src/capture/extract.rs
git commit -m "$(cat <<'EOF'
feat(ccs-proxy): add extract_cwd from system prompt

Parses 'Primary working directory: <path>' line out of Claude Code's
system prompt, handling both string and block-list shapes. Anchored
to line start so prose mentions of 'working directory' don't match.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Extend `SessionMeta` with `cwd` + `models`

**Files:**
- Modify: `ccs-proxy/src/store/mod.rs:11-22`
- Test: `ccs-proxy/tests/store_fs.rs` (append new test)

- [ ] **Step 1: Write the failing backward-compat test**

Append to `ccs-proxy/tests/store_fs.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p ccs-proxy --test store_fs list_sessions_reads_old_meta -- --nocapture`
Expected: compile error — `cwd` / `models` fields don't exist on `SessionMeta`.

- [ ] **Step 3: Add the fields**

Edit `ccs-proxy/src/store/mod.rs` — change the existing `SessionMeta` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub session_id: String,
    pub provider: String,
    pub upstream: String,
    pub proxy_port: u16,
    pub api_port: u16,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub request_count: u64,
    pub schema_version: u32,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub models: Vec<String>,
}
```

- [ ] **Step 4: Run tests to verify it passes**

Run: `cargo test -p ccs-proxy --test store_fs -- --nocapture`
Expected: 3 tests pass (`writes_and_reads_back_records`, `missing_session_returns_none`, new `list_sessions_reads_old_meta_without_cwd_or_models`).

Then run: `cargo check --workspace`
Expected: any other call site that constructs `SessionMeta` still compiles because both new fields default. If it doesn't, fix the call sites by adding `cwd: None, models: vec![]` to the literal — there should not be any since `..Default::default()` isn't used; `SessionMeta` doesn't derive `Default`. Likely callers all use struct-literal syntax. Tests in `tests/daemon_aggregate.rs` and `ccs-proxy/tests/*` will need the two new fields added. Add them with `cwd: None, models: vec![]` everywhere that fails to compile.

- [ ] **Step 5: Commit**

```bash
git add ccs-proxy/src/store/mod.rs ccs-proxy/tests/store_fs.rs tests/daemon_aggregate.rs ccs-proxy/tests/api_sessions.rs
git commit -m "$(cat <<'EOF'
feat(ccs-proxy): add cwd + models fields to SessionMeta

Both #[serde(default)] for backward compatibility with existing
on-disk meta.json files. Populated lazily by FsStore::append in
the next commit.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Backfill `cwd` + `models` in `FsStore::append`

**Files:**
- Modify: `ccs-proxy/src/store/fs.rs:75-88, 133-154`
- Test: `ccs-proxy/tests/store_fs.rs`

- [ ] **Step 1: Write the failing tests**

Append to `ccs-proxy/tests/store_fs.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p ccs-proxy --test store_fs append_ -- --nocapture`
Expected: tests run but assertions fail — `cwd` still `None`, `models` still empty after `append`.

- [ ] **Step 3: Replace `bump_request_count` with `bump_meta`**

Edit `ccs-proxy/src/store/fs.rs`. Find this method around line 75:

```rust
    async fn bump_request_count(&self, session_id: &str, seq: u64) {
        let meta_path = self.meta_path(session_id);
        let Ok(meta_bytes) = fs::read(&meta_path).await else {
            return;
        };
        let Ok(mut meta) = serde_json::from_slice::<SessionMeta>(&meta_bytes) else {
            return;
        };
        meta.request_count = meta.request_count.max(seq);
        let Ok(out) = serde_json::to_vec_pretty(&meta) else {
            return;
        };
        let _ = self.atomic_write(&meta_path, &out).await;
    }
```

Replace it with:

```rust
    async fn bump_meta(&self, rec: &CaptureRecord) {
        let meta_path = self.meta_path(&rec.session_id);
        let Ok(meta_bytes) = fs::read(&meta_path).await else {
            return;
        };
        let Ok(mut meta) = serde_json::from_slice::<SessionMeta>(&meta_bytes) else {
            return;
        };
        meta.request_count = meta.request_count.max(rec.seq);
        if meta.cwd.is_none()
            && let Some(found) = crate::capture::extract::extract_cwd(&rec.request.body)
        {
            meta.cwd = Some(found);
        }
        if let Some(m) = rec.model.as_deref()
            && !meta.models.iter().any(|existing| existing == m)
        {
            meta.models.push(m.to_string());
        }
        let Ok(out) = serde_json::to_vec_pretty(&meta) else {
            return;
        };
        let _ = self.atomic_write(&meta_path, &out).await;
    }
```

Then update the single call site inside `append` (around line 146):

```rust
            Ok(()) => {
                self.note_write_success();
                self.bump_meta(&rec).await;
                Ok(())
            }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p ccs-proxy --test store_fs -- --nocapture`
Expected: all 6 tests pass (3 existing + 3 new).

Also run: `cargo test -p ccs-proxy --lib`
Expected: all existing library tests pass.

- [ ] **Step 5: Commit**

```bash
git add ccs-proxy/src/store/fs.rs ccs-proxy/tests/store_fs.rs
git commit -m "$(cat <<'EOF'
feat(ccs-proxy): backfill cwd + models in FsStore::append

Replaces bump_request_count with bump_meta which, in the same
read+atomic_write, also extracts cwd from the system prompt (first
time only) and appends the request's model to the dedup'd list.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: `/api/sessions` filter params and response fields

**Files:**
- Modify: `src/daemon/aggregate/routes.rs:27-33, 67-113`
- Test: `tests/daemon_aggregate.rs`

- [ ] **Step 1: Write the failing tests**

Append to the `#[cfg(unix)] mod daemon_aggregate` block in `tests/daemon_aggregate.rs` (the existing `aggregate_sessions_merges_across_stores` test will be modified separately; do NOT delete it):

```rust
    #[tokio::test]
    async fn aggregate_sessions_hides_empty_by_default() {
        let tmp = TempDir::new().unwrap();
        let store = Arc::new(FsStore::open(tmp.path().to_path_buf()).unwrap());
        // session A: empty
        store
            .init_session(SessionMeta {
                session_id: "empty_one".to_string(),
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
        // session B: has 1 request
        store
            .init_session(SessionMeta {
                session_id: "has_one".to_string(),
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

        let stores = vec![("https://a.example.com".to_string(), store)];
        let alias_map = Arc::new(cc_switch::daemon::aggregate::state::AliasMap::from_entries(
            vec![],
        ));
        let handle = cc_switch::daemon::aggregate::serve(stores, vec![], alias_map, 0)
            .await
            .unwrap();

        // default: empty hidden
        let resp = reqwest::get(format!("http://127.0.0.1:{}/api/sessions", handle.port))
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["session_id"], "has_one");

        // include_empty=true: both visible
        let resp = reqwest::get(format!(
            "http://127.0.0.1:{}/api/sessions?include_empty=true",
            handle.port
        ))
        .await
        .unwrap();
        let sessions: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(sessions.len(), 2);

        handle.shutdown().await;
    }

    #[tokio::test]
    async fn aggregate_sessions_filters_by_model_and_cwd() {
        let tmp = TempDir::new().unwrap();
        let store = Arc::new(FsStore::open(tmp.path().to_path_buf()).unwrap());

        store
            .init_session(SessionMeta {
                session_id: "s_opus".to_string(),
                provider: "claude".to_string(),
                upstream: "https://a.example.com".to_string(),
                proxy_port: 0,
                api_port: 0,
                started_at: chrono::Utc::now(),
                ended_at: None,
                request_count: 1,
                schema_version: 1,
                cwd: Some("/p/a".to_string()),
                models: vec!["claude-opus-4-7".to_string()],
            })
            .await
            .unwrap();
        store
            .init_session(SessionMeta {
                session_id: "s_sonnet".to_string(),
                provider: "claude".to_string(),
                upstream: "https://a.example.com".to_string(),
                proxy_port: 0,
                api_port: 0,
                started_at: chrono::Utc::now(),
                ended_at: None,
                request_count: 1,
                schema_version: 1,
                cwd: Some("/p/b".to_string()),
                models: vec!["claude-sonnet-4-6".to_string()],
            })
            .await
            .unwrap();
        store
            .init_session(SessionMeta {
                session_id: "s_nocwd".to_string(),
                provider: "claude".to_string(),
                upstream: "https://a.example.com".to_string(),
                proxy_port: 0,
                api_port: 0,
                started_at: chrono::Utc::now(),
                ended_at: None,
                request_count: 1,
                schema_version: 1,
                cwd: None,
                models: vec!["claude-opus-4-7".to_string()],
            })
            .await
            .unwrap();

        let stores = vec![("https://a.example.com".to_string(), store)];
        let alias_map = Arc::new(cc_switch::daemon::aggregate::state::AliasMap::from_entries(
            vec![],
        ));
        let handle = cc_switch::daemon::aggregate::serve(stores, vec![], alias_map, 0)
            .await
            .unwrap();

        // model filter
        let resp = reqwest::get(format!(
            "http://127.0.0.1:{}/api/sessions?model=claude-opus-4-7",
            handle.port
        ))
        .await
        .unwrap();
        let sessions: Vec<serde_json::Value> = resp.json().await.unwrap();
        let ids: Vec<&str> = sessions.iter().map(|s| s["session_id"].as_str().unwrap()).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"s_opus"));
        assert!(ids.contains(&"s_nocwd"));

        // cwd filter
        let resp = reqwest::get(format!(
            "http://127.0.0.1:{}/api/sessions?cwd=/p/a",
            handle.port
        ))
        .await
        .unwrap();
        let sessions: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["session_id"], "s_opus");
        // sanity: response carries cwd + models
        assert_eq!(sessions[0]["cwd"], "/p/a");
        assert_eq!(sessions[0]["models"][0], "claude-opus-4-7");

        // (unknown) cwd matches sessions with cwd: None
        let resp = reqwest::get(format!(
            "http://127.0.0.1:{}/api/sessions?cwd=(unknown)",
            handle.port
        ))
        .await
        .unwrap();
        let sessions: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["session_id"], "s_nocwd");

        // combined: model AND cwd
        let resp = reqwest::get(format!(
            "http://127.0.0.1:{}/api/sessions?model=claude-opus-4-7&cwd=/p/a",
            handle.port
        ))
        .await
        .unwrap();
        let sessions: Vec<serde_json::Value> = resp.json().await.unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["session_id"], "s_opus");

        handle.shutdown().await;
    }
```

Also update the existing `aggregate_sessions_merges_across_stores` test: both `init_session` calls already pass `request_count: 1` and `2` (>0), so they won't be hidden by the new default filter. The only adjustment needed is adding `cwd: None, models: vec![]` to both `SessionMeta` literals (already required after Task 2). No assertion changes.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test daemon_aggregate aggregate_sessions_ -- --nocapture`
Expected: new tests fail — query params not recognized, all sessions returned (no filtering).

- [ ] **Step 3: Implement filters and response fields**

Edit `src/daemon/aggregate/routes.rs`.

Replace the `SessionsQuery` struct (around line 27):

```rust
#[derive(Deserialize, Default)]
struct SessionsQuery {
    upstream: Option<String>,
    alias: Option<String>,
    model: Option<String>,
    cwd: Option<String>,
    include_empty: Option<bool>,
    limit: Option<usize>,
    offset: Option<usize>,
}
```

Replace `list_sessions` (around line 67) entirely with:

```rust
async fn list_sessions(
    State(state): State<SharedState>,
    Query(params): Query<SessionsQuery>,
) -> Json<Value> {
    let mut all_sessions = Vec::new();

    for (upstream, store) in &state.stores {
        let sessions = store.list_sessions().await.unwrap_or_default();
        let aliases = state.alias_map.aliases_for(upstream);
        for session in sessions {
            all_sessions.push(json!({
                "upstream": upstream,
                "aliases": aliases,
                "session_id": session.session_id,
                "provider": session.provider,
                "started_at": session.started_at,
                "ended_at": session.ended_at,
                "request_count": session.request_count,
                "cwd": session.cwd,
                "models": session.models,
            }));
        }
    }

    all_sessions.sort_by(|a, b| {
        let a_time = a["started_at"].as_str().unwrap_or("");
        let b_time = b["started_at"].as_str().unwrap_or("");
        b_time.cmp(a_time)
    });

    if let Some(ref upstream_filter) = params.upstream {
        all_sessions.retain(|s| s["upstream"].as_str() == Some(upstream_filter.as_str()));
    }

    if let Some(ref alias_filter) = params.alias {
        all_sessions.retain(|s| {
            s["aliases"].as_array().is_some_and(|arr| {
                arr.iter()
                    .any(|a| a.as_str() == Some(alias_filter.as_str()))
            })
        });
    }

    if let Some(ref model_csv) = params.model {
        let wanted: Vec<&str> = model_csv.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if !wanted.is_empty() {
            all_sessions.retain(|s| {
                s["models"].as_array().is_some_and(|arr| {
                    arr.iter().any(|m| {
                        m.as_str().is_some_and(|mm| wanted.iter().any(|w| *w == mm))
                    })
                })
            });
        }
    }

    if let Some(ref cwd_csv) = params.cwd {
        let wanted: Vec<&str> = cwd_csv.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if !wanted.is_empty() {
            all_sessions.retain(|s| {
                let session_cwd = s["cwd"].as_str();
                wanted.iter().any(|w| match (*w, session_cwd) {
                    ("(unknown)", None) => true,
                    (w, Some(c)) if w == c => true,
                    _ => false,
                })
            });
        }
    }

    if !params.include_empty.unwrap_or(false) {
        all_sessions.retain(|s| s["request_count"].as_u64().unwrap_or(0) > 0);
    }

    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(100);
    let paginated: Vec<_> = all_sessions.into_iter().skip(offset).take(limit).collect();

    Json(json!(paginated))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test daemon_aggregate -- --nocapture`
Expected: all aggregate tests pass (the existing 4 plus 2 new ones).

- [ ] **Step 5: Commit**

```bash
git add src/daemon/aggregate/routes.rs tests/daemon_aggregate.rs
git commit -m "$(cat <<'EOF'
feat(daemon): /api/sessions model/cwd/include_empty filters

Adds three query params and surfaces cwd + models in the response.
Empty sessions (request_count==0) are hidden by default; the new
include_empty=true brings them back. The literal '(unknown)' in the
cwd filter matches sessions whose cwd is None.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: New `/api/meta` endpoint

**Files:**
- Modify: `src/daemon/aggregate/routes.rs:17-25, end of file`
- Test: `tests/daemon_aggregate.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/daemon_aggregate.rs`:

```rust
    #[tokio::test]
    async fn aggregate_meta_returns_dedup_sorted_lists() {
        let tmp = TempDir::new().unwrap();
        let store = Arc::new(FsStore::open(tmp.path().to_path_buf()).unwrap());
        store
            .init_session(SessionMeta {
                session_id: "s1".to_string(),
                provider: "claude".to_string(),
                upstream: "https://a.example.com".to_string(),
                proxy_port: 0,
                api_port: 0,
                started_at: chrono::Utc::now(),
                ended_at: None,
                request_count: 1,
                schema_version: 1,
                cwd: Some("/p/b".to_string()),
                models: vec!["claude-sonnet-4-6".to_string(), "claude-opus-4-7".to_string()],
            })
            .await
            .unwrap();
        store
            .init_session(SessionMeta {
                session_id: "s2".to_string(),
                provider: "claude".to_string(),
                upstream: "https://a.example.com".to_string(),
                proxy_port: 0,
                api_port: 0,
                started_at: chrono::Utc::now(),
                ended_at: None,
                request_count: 1,
                schema_version: 1,
                cwd: Some("/p/a".to_string()),
                models: vec!["claude-opus-4-7".to_string()],
            })
            .await
            .unwrap();

        let stores = vec![("https://a.example.com".to_string(), store)];
        let alias_map = Arc::new(cc_switch::daemon::aggregate::state::AliasMap::from_entries(
            vec![],
        ));
        let handle = cc_switch::daemon::aggregate::serve(stores, vec![], alias_map, 0)
            .await
            .unwrap();

        let resp = reqwest::get(format!("http://127.0.0.1:{}/api/meta", handle.port))
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();
        let models: Vec<&str> = body["models"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        let cwds: Vec<&str> = body["cwds"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(models, vec!["claude-opus-4-7", "claude-sonnet-4-6"]);
        assert_eq!(cwds, vec!["/p/a", "/p/b"]);

        handle.shutdown().await;
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test daemon_aggregate aggregate_meta_ -- --nocapture`
Expected: 404 from the route — endpoint doesn't exist.

- [ ] **Step 3: Add the route and handler**

Edit `src/daemon/aggregate/routes.rs`. Find the `router()` function (around line 17) and add the `/api/meta` route:

```rust
pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions/{sid}", get(get_session))
        .route("/api/requests/{sid}/{seq}", get(get_request))
        .route("/api/stats", get(stats))
        .route("/api/stream", get(stream))
        .route("/api/meta", get(meta))
}
```

Then append this handler at the end of the file:

```rust
async fn meta(State(state): State<SharedState>) -> Json<Value> {
    use std::collections::BTreeSet;
    let mut models: BTreeSet<String> = BTreeSet::new();
    let mut cwds: BTreeSet<String> = BTreeSet::new();
    for (_upstream, store) in &state.stores {
        let sessions = store.list_sessions().await.unwrap_or_default();
        for s in sessions {
            for m in s.models {
                models.insert(m);
            }
            if let Some(c) = s.cwd {
                cwds.insert(c);
            }
        }
    }
    Json(json!({
        "models": models.into_iter().collect::<Vec<_>>(),
        "cwds": cwds.into_iter().collect::<Vec<_>>(),
    }))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test daemon_aggregate -- --nocapture`
Expected: all 6 aggregate tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/daemon/aggregate/routes.rs tests/daemon_aggregate.rs
git commit -m "$(cat <<'EOF'
feat(daemon): /api/meta endpoint with deduped models + cwds

Backs the sidebar's Model and Working-dirs filter blocks. Uses
BTreeSet for sort+dedup in one pass; reads only meta.json files
across all stores (no record-file I/O).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: Frontend HTML scaffolding for new filter blocks

**Files:**
- Modify: `web-aggregate/index.html:26-35`

- [ ] **Step 1: Insert the new filter blocks**

Edit `web-aggregate/index.html`. Find the existing filter section (lines 26-35) and replace it with:

```html
      <h3>Upstreams</h3>
      <div id="upstream-filters"></div>
      <h3>Models</h3>
      <div id="model-filters"></div>
      <h3>Working dirs</h3>
      <div id="cwd-filters"></div>
      <h3>Time</h3>
      <div id="time-filters">
        <button class="time-btn active" data-window="1h">1h</button>
        <button class="time-btn" data-window="24h">24h</button>
        <button class="time-btn" data-window="7d">7d</button>
        <button class="time-btn" data-window="all">all</button>
      </div>
      <label id="show-empty-wrap" class="hidden">
        <input type="checkbox" id="show-empty-sessions">
        Show empty sessions
      </label>
```

- [ ] **Step 2: Add CSS for the new blocks**

Edit `web-aggregate/style.css`. Append at the end:

```css
#model-filters label,
#cwd-filters label {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 0;
  cursor: pointer;
  font-size: 12px;
}

#cwd-filters label .cwd-path {
  margin-left: auto;
  color: var(--muted);
  font-size: 10px;
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

#show-empty-wrap {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 0;
  cursor: pointer;
  font-size: 12px;
}
```

- [ ] **Step 3: Smoke-build to make sure RustEmbed still picks up assets**

Run: `cargo build`
Expected: clean build.

- [ ] **Step 4: Commit**

```bash
git add web-aggregate/index.html web-aggregate/style.css
git commit -m "$(cat <<'EOF'
feat(web-aggregate): sidebar scaffolding for model + cwd filters

Empty containers + 'Show empty sessions' toggle. Wired up by JS in
the next commit.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: Frontend `loadMeta` + filter rendering + wiring

**Files:**
- Modify: `web-aggregate/app.js:1-120, 237-260, 470-490`

- [ ] **Step 1: Add new global state at the top of `app.js`**

In `web-aggregate/app.js`, find the top-level `let`/`const` block (lines 3-17) and replace it with:

```javascript
const COLORS = ['#0066cc', '#e65100', '#2e7d32', '#6a1b9a', '#c62828', '#00838f'];
let upstreamColors = {};
let activeUpstreams = new Set();
let activeModels = new Set();
let activeCwds = new Set();
let availableModels = [];
let availableCwds = [];
let showEmptySessions = false;
let timeWindow = '1h';
let currentPage = 0;
const PAGE_SIZE = 50;
let allRequests = [];
let healthData = null;
let viewMode = 'requests';
let allSessions = [];
let sessionPage = 0;
const SESSION_PAGE_SIZE = 50;
let activeSessionId = null;
let sessionRequests = [];
let sessionReqPage = 0;
const UNKNOWN_CWD = '(unknown)';
```

- [ ] **Step 2: Update `init()` to fetch and render meta**

Replace the existing `init()` function (around line 19) with:

```javascript
async function init() {
  try {
    const resp = await fetch('/api/health');
    healthData = await resp.json();
    renderHeader();
    renderUpstreamFilters();
    await loadMeta();
    await loadStats();
    await loadRequests();
    connectStream();
  } catch (e) {
    document.getElementById('meta').textContent = 'connection failed';
  }
}
```

Then append a new `loadMeta()` function right after `init()`:

```javascript
async function loadMeta() {
  try {
    const resp = await fetch('/api/meta');
    const data = await resp.json();
    availableModels = data.models || [];
    availableCwds = [...(data.cwds || []), UNKNOWN_CWD];
    activeModels = new Set(availableModels);
    activeCwds = new Set(availableCwds);
    renderModelFilters();
    renderCwdFilters();
  } catch (e) {
    // Fall back to no model/cwd filters; rest of dashboard still works.
  }
}

function renderModelFilters() {
  const container = document.getElementById('model-filters');
  container.innerHTML = '';
  for (const model of availableModels) {
    const label = document.createElement('label');
    const cb = document.createElement('input');
    cb.type = 'checkbox';
    cb.checked = activeModels.has(model);
    cb.addEventListener('change', () => {
      if (cb.checked) activeModels.add(model);
      else activeModels.delete(model);
      reloadCurrentView();
    });
    label.appendChild(cb);
    label.appendChild(document.createTextNode(` ${model}`));
    container.appendChild(label);
  }
}

function renderCwdFilters() {
  const container = document.getElementById('cwd-filters');
  container.innerHTML = '';
  for (const cwd of availableCwds) {
    const label = document.createElement('label');
    const cb = document.createElement('input');
    cb.type = 'checkbox';
    cb.checked = activeCwds.has(cwd);
    cb.addEventListener('change', () => {
      if (cb.checked) activeCwds.add(cwd);
      else activeCwds.delete(cwd);
      reloadCurrentView();
    });
    const basename = cwd === UNKNOWN_CWD
      ? UNKNOWN_CWD
      : cwd.split('/').filter(Boolean).pop() || cwd;
    const path = document.createElement('span');
    path.className = 'cwd-path';
    path.textContent = cwd === UNKNOWN_CWD ? '' : cwd;
    path.title = cwd;
    label.appendChild(cb);
    label.appendChild(document.createTextNode(` ${basename}`));
    label.appendChild(path);
    container.appendChild(label);
  }
}

function reloadCurrentView() {
  currentPage = 0;
  sessionPage = 0;
  if (viewMode === 'sessions' && !activeSessionId) {
    loadSessions();
  } else if (viewMode === 'sessions' && activeSessionId) {
    // user is drilled into a session — leave it alone
  } else {
    loadRequests();
  }
}
```

- [ ] **Step 3: Build query strings and route requests through them**

Inside `app.js`, find `loadRequests()` (around line 97) and replace it with:

```javascript
function buildFilterQuery({ includeEmpty } = {}) {
  const parts = [];
  if (activeUpstreams.size < (healthData?.proxies?.length ?? 0)) {
    // Sidebar de-selected some; the API uses one upstream= param at a time
    // — fall back to client filtering for upstream (kept as is in the
    // existing renderers). Skip server-side upstream filtering.
  }
  if (activeModels.size && activeModels.size < availableModels.length) {
    parts.push('model=' + encodeURIComponent([...activeModels].join(',')));
  }
  if (activeCwds.size && activeCwds.size < availableCwds.length) {
    parts.push('cwd=' + encodeURIComponent([...activeCwds].join(',')));
  }
  if (includeEmpty) parts.push('include_empty=true');
  return parts.length ? '?' + parts.join('&') : '';
}

async function loadRequests() {
  try {
    const qs = buildFilterQuery({ includeEmpty: false });
    const sep = qs ? '&' : '?';
    const resp = await fetch(`/api/sessions${qs}${sep}limit=200`);
    const sessions = await resp.json();
    allRequests = [];
    for (const session of sessions) {
      const rResp = await fetch(`/api/sessions/${session.session_id}`);
      const detail = await rResp.json();
      for (const req of (detail.requests || [])) {
        allRequests.push({
          ...req,
          upstream: session.upstream,
          aliases: session.aliases,
          cwd: session.cwd,
        });
      }
    }
    allRequests.sort((a, b) => new Date(b.started_at) - new Date(a.started_at));
    renderRequestTable();
  } catch (e) { /* ignore */ }
}
```

Find `loadSessions()` (around line 237) and replace it with:

```javascript
async function loadSessions() {
  try {
    const qs = buildFilterQuery({ includeEmpty: showEmptySessions });
    const sep = qs ? '&' : '?';
    const resp = await fetch(`/api/sessions${qs}${sep}limit=500`);
    allSessions = await resp.json();
    sessionPage = 0;
    activeSessionId = null;
    showSessionList();
  } catch (e) { /* ignore */ }
}
```

- [ ] **Step 4: Per-request model filter in Requests view**

Find `renderRequestTable()` (around line 118). Inside its `.filter(...)` chain, add a model check. Replace the filter block:

```javascript
  const filtered = allRequests.filter(r => {
    if (!activeUpstreams.has(r.upstream)) return false;
    if (since && new Date(r.started_at) < since) return false;
    return true;
  });
```

with:

```javascript
  const filtered = allRequests.filter(r => {
    if (!activeUpstreams.has(r.upstream)) return false;
    if (since && new Date(r.started_at) < since) return false;
    if (activeModels.size && activeModels.size < availableModels.length) {
      if (!r.model || !activeModels.has(r.model)) return false;
    }
    return true;
  });
```

- [ ] **Step 5: Wire up the "Show empty sessions" checkbox and show it conditionally**

In `app.js`, find `switchView()` (around line 218) and replace it with:

```javascript
function switchView(mode) {
  viewMode = mode;
  document.querySelectorAll('.view-btn').forEach(b => b.classList.remove('active'));
  document.querySelector(`.view-btn[data-view="${mode}"]`)?.classList.add('active');

  const requestsView = document.getElementById('requests-view');
  const sessionView = document.getElementById('session-view');
  const emptyWrap = document.getElementById('show-empty-wrap');

  if (mode === 'sessions') {
    requestsView.classList.add('hidden');
    sessionView.classList.remove('hidden');
    emptyWrap.classList.remove('hidden');
    loadSessions();
  } else {
    requestsView.classList.remove('hidden');
    sessionView.classList.add('hidden');
    emptyWrap.classList.add('hidden');
    activeSessionId = null;
  }
}
```

At the bottom of `app.js`, right after the existing "Back to sessions button" line, add:

```javascript
// Show-empty-sessions toggle
document.getElementById('show-empty-sessions')?.addEventListener('change', (e) => {
  showEmptySessions = e.target.checked;
  loadSessions();
});
```

- [ ] **Step 6: Manual smoke test**

```bash
cargo build --release
# Make sure the daemon is running with captured data
./target/release/cc-switch daemon status
# (if not running: ./target/release/cc-switch daemon start)
```

Open the dashboard URL from `daemon status` in a browser. Verify:

- Sidebar shows "Models" block with checkboxes for every model seen.
- Sidebar shows "Working dirs" block with basenames + greyed full paths.
- "Show empty sessions" only appears when on the Sessions view.
- Toggling a model checkbox immediately filters the request rows (Requests view) and session rows (Sessions view).
- Toggling a cwd checkbox does the same.
- Sessions view by default hides empty sessions. Click "Show empty sessions" → ghost sessions reappear; the previously-empty click bug is no longer reachable in default state.
- Click a non-empty session in the Sessions view → drills in, shows requests, click one → right-side detail panel opens.

- [ ] **Step 7: Commit**

```bash
git add web-aggregate/app.js
git commit -m "$(cat <<'EOF'
feat(web-aggregate): wire model + cwd filters and empty-toggle

loadMeta() fetches /api/meta and renders dynamic sidebar checkboxes
for both filters. loadSessions/loadRequests build a query string from
active sets. renderRequestTable adds a per-request model filter pass
since server-side filtering matches at session granularity.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: SSE live update for new model / cwd

**Files:**
- Modify: `web-aggregate/app.js:400-460` (`connectStream` + `prependLiveRow`)

- [ ] **Step 1: Apply live filters in `prependLiveRow`**

In `app.js`, replace the existing `prependLiveRow(data, type)` function with:

```javascript
function prependLiveRow(data, type) {
  const tbody = document.getElementById('request-list');
  if (!activeUpstreams.has(data.upstream)) return;
  if (activeModels.size && availableModels.length && activeModels.size < availableModels.length) {
    if (data.model && !activeModels.has(data.model)) return;
  }

  // If a brand-new model arrives, register it and re-render the sidebar.
  if (data.model && !availableModels.includes(data.model)) {
    availableModels.push(data.model);
    activeModels.add(data.model);
    renderModelFilters();
  }

  const tr = document.createElement('tr');
  const color = upstreamColors[data.upstream] || '#999';
  tr.style.borderLeftColor = color;
  tr.className = 'new-row';

  if (type === 'completed') {
    tr.className += data.has_error ? ' status-error' : ' status-ok';
    const tokens = (data.usage?.input_tokens || 0) + (data.usage?.output_tokens || 0);
    tr.innerHTML = `
      <td>${new Date().toLocaleTimeString()}</td>
      <td>${data.upstream.replace(/^https?:\/\//, '').slice(0, 25)}</td>
      <td>—</td>
      <td>${tokens || '—'}</td>
      <td>${data.status}</td>
      <td>${data.duration_ms}ms</td>
    `;
  } else {
    tr.innerHTML = `
      <td>${new Date().toLocaleTimeString()}</td>
      <td>${data.upstream.replace(/^https?:\/\//, '').slice(0, 25)}</td>
      <td>${data.model || '—'}</td>
      <td>—</td>
      <td>...</td>
      <td>—</td>
    `;
  }

  tr.addEventListener('click', () => selectRow(data.session_id, data.seq));
  tbody.prepend(tr);

  while (tbody.children.length > PAGE_SIZE) {
    tbody.removeChild(tbody.lastChild);
  }
}
```

- [ ] **Step 2: Manual smoke test**

While the dashboard is open and the daemon is capturing live traffic:
- Trigger a request with a model that wasn't yet in the sidebar (e.g., switch Claude Code config to use Haiku for one prompt).
- Verify the new model appears in the sidebar and is auto-selected.
- Uncheck that model and verify subsequent live rows for it are suppressed.

- [ ] **Step 3: Commit**

```bash
git add web-aggregate/app.js
git commit -m "$(cat <<'EOF'
feat(web-aggregate): SSE prepend respects filters + registers new models

Live rows now pass through the active model filter; a previously-
unseen model auto-registers in the sidebar (default-checked) so
subsequent rows continue to flow.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: Full regression run + final manual check

**Files:** (none — verification only)

- [ ] **Step 1: Run the full test suite**

Run: `cargo test --workspace`
Expected: every test passes. If any test in `tests/daemon_aggregate.rs`, `ccs-proxy/tests/*`, or `src/` fails because it constructs a `SessionMeta` without `cwd` / `models`, add `cwd: None, models: vec![]` to that literal — same fix pattern from Task 2 step 4.

- [ ] **Step 2: Run lint + format checks**

Run: `cargo fmt --check && cargo clippy --workspace -- -D warnings`
Expected: clean.

- [ ] **Step 3: Final manual end-to-end pass**

Restart the daemon to load the new binary:

```bash
./target/release/cc-switch daemon stop
cargo build --release
./target/release/cc-switch daemon start
```

Open the dashboard. Verify all three original issues are resolved:

1. **Model filter**: Sidebar shows Models checkboxes; toggling filters both Requests and Sessions views.
2. **Session click**: With default settings, the Sessions view no longer lists ghost sessions; clicking a real session drills in and shows requests.
3. **cwd filter**: Sidebar shows Working dirs; toggling filters both views. Sessions captured before this change show `(unknown)` until a new request is appended (which will backfill the cwd).

- [ ] **Step 4: Final commit if any small fixes were needed in Steps 1-3**

```bash
git status
# if anything changed:
git add -p
git commit -m "..."
```

If everything was clean, skip this step — no empty commit.

---

## Self-review checklist (done by the planner — not part of execution)

- [x] Every spec requirement maps to a task: model filter (Tasks 5, 7), cwd filter (Tasks 1-3, 7), empty-session hide (Task 4 default + Task 7 toggle), backward-compat read (Task 2), extraction tests (Task 1), API tests (Tasks 4-5), per-request model filter (Task 7 step 4), SSE update (Task 8).
- [x] No "TBD"/"TODO"/"similar to" — every step has the actual code.
- [x] Method names consistent: `bump_meta`, `extract_cwd`, `loadMeta`, `buildFilterQuery`, `renderModelFilters`, `renderCwdFilters`, `reloadCurrentView` used identically in every place they appear.
- [x] Type signatures consistent: `cwd: Option<String>` and `models: Vec<String>` on `SessionMeta` everywhere; JSON shape `{ "cwd": null | string, "models": [...] }`.
