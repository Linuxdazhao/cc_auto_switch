//! Reverse-proxy app: matches every inbound HTTP request via a fallback
//! handler, forwards it to the configured upstream, and tees a copy of the
//! response byte stream into a background reassembler for capture.

pub mod forward;
pub mod sse_tap;

use crate::AppState;
use axum::Router;
use axum::routing::{any, get};

pub fn build_proxy_app(state: AppState) -> Router {
    Router::new()
        .route("/", get(|| async { "ccs-proxy: send requests here" }))
        .fallback(any(forward::forward))
        .with_state(state)
}
