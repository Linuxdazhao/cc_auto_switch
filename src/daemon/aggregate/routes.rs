use super::state::AggregateState;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::{Json, Router, routing::get};
use ccs_proxy::store::Store;
use futures::stream::Stream;
use serde::Deserialize;
use serde_json::{Value, json};
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

type SharedState = Arc<AggregateState>;

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

#[derive(Deserialize, Default)]
struct StatsQuery {
    since: Option<String>,
}

async fn health(State(state): State<SharedState>) -> Json<Value> {
    let uptime_s = (chrono::Utc::now() - state.started_at).num_seconds();
    let mut total_requests: u64 = 0;
    let mut proxies = Vec::new();

    for (upstream, store) in &state.stores {
        let sessions = store.list_sessions().await.unwrap_or_default();
        let req_count: u64 = sessions.iter().map(|s| s.request_count).sum();
        total_requests += req_count;
        let aliases = state.alias_map.aliases_for(upstream);
        proxies.push(json!({
            "upstream": upstream,
            "request_count": req_count,
            "aliases": aliases,
        }));
    }

    Json(json!({
        "status": "ok",
        "pid": std::process::id(),
        "uptime_s": uptime_s,
        "proxy_count": state.stores.len(),
        "total_requests": total_requests,
        "proxies": proxies,
    }))
}

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
        let wanted: Vec<&str> = model_csv
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if !wanted.is_empty() {
            all_sessions.retain(|s| {
                s["models"].as_array().is_some_and(|arr| {
                    arr.iter()
                        .any(|m| m.as_str().is_some_and(|mm| wanted.contains(&mm)))
                })
            });
        }
    }

    if let Some(ref cwd_csv) = params.cwd {
        let wanted: Vec<&str> = cwd_csv
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
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

async fn get_session(
    State(state): State<SharedState>,
    Path(sid): Path<String>,
) -> axum::response::Response {
    for (upstream, store) in &state.stores {
        let sessions = store.list_sessions().await.unwrap_or_default();
        if let Some(meta) = sessions.into_iter().find(|s| s.session_id == sid) {
            let requests = store.list_requests(&sid).await.unwrap_or_default();
            let aliases = state.alias_map.aliases_for(upstream);
            return Json(json!({
                "upstream": upstream,
                "aliases": aliases,
                "meta": meta,
                "requests": requests,
            }))
            .into_response();
        }
    }
    (axum::http::StatusCode::NOT_FOUND, "session not found").into_response()
}

#[allow(clippy::type_complexity)]
async fn get_request(
    State(state): State<SharedState>,
    Path((sid, seq)): Path<(String, u64)>,
) -> axum::response::Response {
    for (_upstream, store) in &state.stores {
        if let Ok(Some(rec)) = store.get_request(&sid, seq).await {
            return match serde_json::to_value(&rec) {
                Ok(val) => Json(val).into_response(),
                Err(_) => (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "serialize error",
                )
                    .into_response(),
            };
        }
    }
    (axum::http::StatusCode::NOT_FOUND, "request not found").into_response()
}

async fn stats(State(state): State<SharedState>, Query(params): Query<StatsQuery>) -> Json<Value> {
    let since = params
        .since
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let mut upstreams_stats = Vec::new();
    let mut totals_requests: u64 = 0;
    let mut totals_input: u64 = 0;
    let mut totals_output: u64 = 0;
    let mut totals_duration: u64 = 0;
    let mut totals_duration_count: u64 = 0;
    let mut totals_errors: u64 = 0;

    for (upstream, store) in &state.stores {
        let sessions = store.list_sessions().await.unwrap_or_default();
        let aliases = state.alias_map.aliases_for(upstream);
        let mut total_req: u64 = 0;
        let mut input_tokens: u64 = 0;
        let mut output_tokens: u64 = 0;
        let mut duration_sum: u64 = 0;
        let mut duration_count: u64 = 0;
        let mut error_count: u64 = 0;
        let session_count = sessions.len() as u64;

        for session in &sessions {
            let requests = store
                .list_requests(&session.session_id)
                .await
                .unwrap_or_default();
            for req in &requests {
                if since.is_some_and(|since_dt| req.started_at < since_dt) {
                    continue;
                }
                total_req += 1;
                if let Some(inp) = req.input_tokens {
                    input_tokens += inp;
                }
                if let Some(outp) = req.output_tokens {
                    output_tokens += outp;
                }
                if let Some(dur) = req.duration_ms {
                    duration_sum += dur;
                    duration_count += 1;
                }
                if req.has_error {
                    error_count += 1;
                }
            }
        }

        let avg_duration = duration_sum.checked_div(duration_count).unwrap_or(0);
        let error_rate = if total_req > 0 {
            error_count as f64 / total_req as f64
        } else {
            0.0
        };

        totals_requests += total_req;
        totals_input += input_tokens;
        totals_output += output_tokens;
        totals_duration += duration_sum;
        totals_duration_count += duration_count;
        totals_errors += error_count;

        upstreams_stats.push(json!({
            "upstream": upstream,
            "aliases": aliases,
            "total_requests": total_req,
            "total_input_tokens": input_tokens,
            "total_output_tokens": output_tokens,
            "avg_duration_ms": avg_duration,
            "error_count": error_count,
            "error_rate": error_rate,
            "session_count": session_count,
        }));
    }

    let totals_avg_duration = totals_duration
        .checked_div(totals_duration_count)
        .unwrap_or(0);

    Json(json!({
        "upstreams": upstreams_stats,
        "totals": {
            "total_requests": totals_requests,
            "total_input_tokens": totals_input,
            "total_output_tokens": totals_output,
            "avg_duration_ms": totals_avg_duration,
            "error_count": totals_errors,
        },
        "since": since,
    }))
}

async fn stream(
    State(state): State<SharedState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.merged_events.subscribe();
    let sse_stream = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(tagged) => {
            let event_name = match &tagged.inner {
                ccs_proxy::CaptureEvent::RequestStarted { .. } => "request_started",
                ccs_proxy::CaptureEvent::RequestCompleted { .. } => "request_completed",
            };
            let data = serde_json::to_string(&tagged).unwrap_or_default();
            Some(Ok(Event::default().event(event_name).data(data)))
        }
        Err(_) => None,
    });
    Sse::new(sse_stream).keep_alive(KeepAlive::default())
}

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
