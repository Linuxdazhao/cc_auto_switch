//! Reqwest-backed streaming reverse-proxy handler.
//!
//! Every inbound HTTP request is forwarded to `state.upstream` unchanged
//! (modulo hop-by-hop headers), the response is streamed back to the client
//! as it arrives (no buffering), and a tee'd copy of the byte stream is fed
//! to a background task that reassembles the SSE into a final JSON message
//! and writes a `CaptureRecord` via `state.store`.

use crate::AppState;
use crate::capture::{CaptureEvent, CaptureRecord, RequestPart, ResponsePart, Usage};
use crate::proxy::sse_tap::{self, TapReceiver};
use axum::body::Body;
use axum::extract::{OriginalUri, Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;
use url::Url;

const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
    "host",
];

const MAX_REQUEST_BODY: usize = 32 * 1024 * 1024;

/// Returns a process-global `reqwest::Client` so that successive forwarded
/// requests reuse the connection pool, DNS cache, and TLS session cache.
/// Rebuilding a `Client` per request defeats keep-alive and adds measurable
/// TTFT overhead for a proxy.
fn upstream_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

/// Headers prepared for the inbound side of the response, plus the same
/// headers projected to a `BTreeMap` for the capture record.
type ResponseHeaderPair = (HeaderMap, BTreeMap<String, String>);

/// Inputs collected from the inbound request before we hit the network.
struct PreparedRequest {
    method: Method,
    upstream_url: Url,
    path_for_capture: String,
    req_headers: HeaderMap,
    body_bytes: Bytes,
    req_body_json: Value,
}

/// All inputs the background capture task needs after the response status +
/// headers are known.
struct CaptureCtx {
    state: AppState,
    started_at: DateTime<Utc>,
    method: Method,
    path: String,
    req_headers_map: BTreeMap<String, String>,
    req_body_json: Value,
    resp_status: u16,
    resp_headers_map: BTreeMap<String, String>,
    model: Option<String>,
}

pub async fn forward(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    req: Request,
) -> Response {
    let method = req.method().clone();
    let req_headers = req.headers().clone();
    let path_for_capture = uri
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| "/".into());
    let upstream_url = build_upstream_url(&state.upstream, &uri);

    let body_bytes = match read_request_body(req).await {
        Ok(bytes) => bytes,
        Err(resp) => return resp,
    };

    let req_body_json = serde_json::from_slice::<Value>(&body_bytes).unwrap_or(Value::Null);
    let prepared = PreparedRequest {
        method,
        upstream_url,
        path_for_capture,
        req_headers,
        body_bytes,
        req_body_json,
    };
    dispatch(state, prepared).await
}

async fn dispatch(state: AppState, prepared: PreparedRequest) -> Response {
    let upstream_resp = match send_upstream(&prepared).await {
        Ok(resp) => resp,
        Err(err_resp) => return err_resp,
    };

    let status = upstream_resp.status();
    let (resp_headers, resp_headers_map) = collect_response_headers(upstream_resp.headers());
    let started_at = chrono::Utc::now();
    let model = crate::capture::extract::extract_model_from_request_body(&prepared.req_body_json);
    let req_headers_map = headers_to_map(&prepared.req_headers);

    let byte_stream = upstream_resp.bytes_stream();
    let (client_stream, tap_rx) = sse_tap::tee(byte_stream);

    let ctx = CaptureCtx {
        state,
        started_at,
        method: prepared.method,
        path: prepared.path_for_capture,
        req_headers_map,
        req_body_json: prepared.req_body_json,
        resp_status: status.as_u16(),
        resp_headers_map,
        model,
    };
    tokio::spawn(run_capture(ctx, tap_rx));

    build_streaming_response(status, resp_headers, client_stream)
}

fn build_upstream_url(upstream: &Url, uri: &axum::http::Uri) -> Url {
    let mut url = upstream.clone();
    url.set_path(uri.path());
    url.set_query(uri.query());
    url
}

async fn read_request_body(req: Request) -> Result<Bytes, Response> {
    match axum::body::to_bytes(req.into_body(), MAX_REQUEST_BODY).await {
        Ok(bytes) => Ok(bytes),
        Err(err) => {
            tracing::warn!(?err, "failed to read request body");
            Err((
                StatusCode::BAD_REQUEST,
                "request body too large or unreadable",
            )
                .into_response())
        }
    }
}

async fn send_upstream(prepared: &PreparedRequest) -> Result<reqwest::Response, Response> {
    let mut rb = upstream_client()
        .request(
            reqwest_method(&prepared.method),
            prepared.upstream_url.clone(),
        )
        .body(prepared.body_bytes.to_vec());
    for (name, value) in prepared.req_headers.iter() {
        let kn = name.as_str();
        if HOP_BY_HOP.iter().any(|h| h.eq_ignore_ascii_case(kn)) {
            continue;
        }
        if kn.eq_ignore_ascii_case("content-length") {
            continue;
        }
        if let (Ok(rname), Ok(rval)) = (
            reqwest::header::HeaderName::from_bytes(name.as_str().as_bytes()),
            reqwest::header::HeaderValue::from_bytes(value.as_bytes()),
        ) {
            rb = rb.header(rname, rval);
        }
    }

    match rb.send().await {
        Ok(resp) => Ok(resp),
        Err(err) => {
            let kind = classify_reqwest_err(&err);
            tracing::warn!(?err, kind, "upstream request failed");
            let body = serde_json::json!({
                "error": {
                    "type": kind,
                    "message": err.to_string(),
                }
            });
            Err((StatusCode::BAD_GATEWAY, axum::Json(body)).into_response())
        }
    }
}

fn collect_response_headers(upstream: &reqwest::header::HeaderMap) -> ResponseHeaderPair {
    let mut axum_headers = HeaderMap::new();
    let mut as_map: BTreeMap<String, String> = BTreeMap::new();
    for (name, value) in upstream.iter() {
        if HOP_BY_HOP
            .iter()
            .any(|h| name.as_str().eq_ignore_ascii_case(h))
        {
            continue;
        }
        if let (Ok(an), Ok(av)) = (
            HeaderName::from_bytes(name.as_str().as_bytes()),
            HeaderValue::from_bytes(value.as_bytes()),
        ) {
            axum_headers.insert(an, av);
        }
        if let Ok(text) = value.to_str() {
            as_map.insert(name.as_str().to_string(), text.to_string());
        }
    }
    (axum_headers, as_map)
}

fn build_streaming_response<S>(status: StatusCode, headers: HeaderMap, client_stream: S) -> Response
where
    S: futures::Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static,
{
    let body = Body::from_stream(client_stream);
    let mut builder = Response::builder().status(status);
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }
    builder
        .body(body)
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}

async fn run_capture(ctx: CaptureCtx, tap_rx: TapReceiver) {
    let CaptureCtx {
        state,
        started_at,
        method,
        path,
        mut req_headers_map,
        mut req_body_json,
        resp_status,
        mut resp_headers_map,
        model,
    } = ctx;

    let request_id = crate::capture::extract::extract_request_id(&resp_headers_map);
    let seq = next_seq(&state.store, state.session_id.as_str()).await;

    if let Err(err) = state.events.send(CaptureEvent::RequestStarted {
        session_id: state.session_id.as_str().to_string(),
        seq,
        started_at,
        model: model.clone(),
    }) {
        tracing::trace!(?err, "no subscribers for RequestStarted");
    }

    let (body_reassembled, frames_count, partial_err) =
        sse_tap::reassemble(state.provider, tap_rx).await;

    let ended_at = chrono::Utc::now();
    let duration_ms = duration_ms_since(started_at, ended_at);
    let usage = usage_from_reassembled(body_reassembled.as_ref());

    if state.redact {
        crate::capture::redact::redact_headers(&mut req_headers_map);
        crate::capture::redact::redact_body(&mut req_body_json);
        crate::capture::redact::redact_headers(&mut resp_headers_map);
    }

    let rec = CaptureRecord {
        seq,
        session_id: state.session_id.as_str().to_string(),
        request_id: request_id.clone(),
        started_at,
        ended_at: Some(ended_at),
        duration_ms: Some(duration_ms),
        ttft_ms: None,
        request: RequestPart {
            method: method.as_str().to_string(),
            path,
            headers: req_headers_map,
            body: req_body_json,
        },
        response: Some(ResponsePart {
            status: resp_status,
            headers: resp_headers_map,
            body_reassembled,
            raw_sse_text: None,
            raw_sse_frames_count: frames_count,
        }),
        usage: usage.clone(),
        model,
        error: partial_err.clone(),
        partial: partial_err.is_some(),
        schema_version: 1,
    };
    if let Err(err) = state.store.append(rec).await {
        tracing::warn!(?err, "store append failed");
    }

    let has_error = partial_err.is_some() || resp_status >= 400;
    if let Err(err) = state.events.send(CaptureEvent::RequestCompleted {
        session_id: state.session_id.as_str().to_string(),
        seq,
        duration_ms,
        status: resp_status,
        request_id,
        usage,
        has_error,
    }) {
        tracing::trace!(?err, "no subscribers for RequestCompleted");
    }
}

fn headers_to_map(headers: &HeaderMap) -> BTreeMap<String, String> {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    for (name, value) in headers.iter() {
        if let Ok(text) = value.to_str() {
            out.insert(name.as_str().to_string(), text.to_string());
        }
    }
    out
}

fn reqwest_method(method: &Method) -> reqwest::Method {
    reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET)
}

fn classify_reqwest_err(err: &reqwest::Error) -> &'static str {
    if err.is_timeout() {
        return "upstream_timeout";
    }
    if err.is_connect() {
        return "upstream_unreachable";
    }
    if err.to_string().to_lowercase().contains("tls") {
        return "tls_handshake_failed";
    }
    "upstream_error"
}

fn usage_from_reassembled(value: Option<&Value>) -> Option<Usage> {
    let value = value?;
    let usage = value.get("usage")?;
    Some(Usage {
        input_tokens: usage
            .get("input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        output_tokens: usage
            .get("output_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cache_creation_input_tokens: usage
            .get("cache_creation_input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cache_read_input_tokens: usage
            .get("cache_read_input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
    })
}

fn duration_ms_since(started_at: DateTime<Utc>, ended_at: DateTime<Utc>) -> u64 {
    let millis = (ended_at - started_at).num_milliseconds().max(0);
    u64::try_from(millis).unwrap_or(0)
}

async fn next_seq(store: &Arc<dyn crate::store::Store>, sid: &str) -> u64 {
    let highest = store
        .list_requests(sid)
        .await
        .map(|list| list.iter().map(|item| item.seq).max().unwrap_or(0))
        .unwrap_or(0);
    highest.saturating_add(1)
}
