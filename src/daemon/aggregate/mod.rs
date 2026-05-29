pub mod routes;
pub mod state;
pub mod stream;

use ccs_proxy::CaptureEvent;
use state::{AggregateState, AliasMap, StoreEntry};
use std::sync::Arc;
use stream::TaggedCaptureEvent;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

pub type EventSenderEntry = (String, broadcast::Sender<CaptureEvent>);

pub struct AggregateHandle {
    pub port: u16,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    join: Option<JoinHandle<()>>,
}

impl AggregateHandle {
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(join) = self.join.take() {
            let _ = join.await;
        }
    }
}

impl Drop for AggregateHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

pub async fn serve(
    stores: Vec<StoreEntry>,
    proxy_events: Vec<EventSenderEntry>,
    alias_map: Arc<AliasMap>,
    port: u16,
) -> anyhow::Result<AggregateHandle> {
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    let bound_port = listener.local_addr()?.port();

    let (merged_tx, _) = broadcast::channel::<TaggedCaptureEvent>(2048);

    let receivers: Vec<_> = proxy_events
        .iter()
        .map(|(upstream, sender)| (upstream.clone(), sender.subscribe()))
        .collect();
    if !receivers.is_empty() {
        let merger_alias_map = alias_map.clone();
        let merger_tx = merged_tx.clone();
        tokio::spawn(stream::event_merger(receivers, merger_alias_map, merger_tx));
    }

    let agg_state = Arc::new(AggregateState {
        stores,
        merged_events: merged_tx,
        alias_map,
        started_at: chrono::Utc::now(),
    });

    let app = axum::Router::new()
        .merge(routes::router())
        .merge(ui_router())
        .with_state(agg_state);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let join = tokio::spawn(async move {
        let server = axum::serve(listener, app);
        tokio::select! {
            res = server => {
                if let Err(err) = res {
                    tracing::warn!(error = %err, "aggregate server exited");
                }
            }
            _ = shutdown_rx => {}
        }
    });

    tracing::info!(port = bound_port, "aggregate server started");

    Ok(AggregateHandle {
        port: bound_port,
        shutdown_tx: Some(shutdown_tx),
        join: Some(join),
    })
}

use axum::Router;
#[cfg(feature = "web-ui")]
use axum::http::{StatusCode, header};
#[cfg(feature = "web-ui")]
use axum::response::{IntoResponse, Response};
#[cfg(feature = "web-ui")]
use axum::routing::get;
#[cfg(feature = "web-ui")]
use rust_embed::RustEmbed;

#[cfg(feature = "web-ui")]
#[derive(RustEmbed)]
#[folder = "web-aggregate/dist/"]
struct AggWebAsset;

#[cfg(feature = "web-ui")]
fn ui_router() -> Router<Arc<AggregateState>> {
    use axum::extract::Path;
    Router::new()
        .route("/", get(|| async { serve_asset("index.html") }))
        .route(
            "/{*path}",
            get(|Path(path): Path<String>| async move {
                if AggWebAsset::get(&path).is_some() {
                    serve_asset(&path)
                } else {
                    serve_asset("index.html")
                }
            }),
        )
}

#[cfg(not(feature = "web-ui"))]
fn ui_router() -> Router<Arc<AggregateState>> {
    Router::new()
}

#[cfg(feature = "web-ui")]
fn serve_asset(name: &str) -> Response {
    match AggWebAsset::get(name) {
        Some(asset) => {
            let mime = match std::path::Path::new(name)
                .extension()
                .and_then(|x| x.to_str())
            {
                Some("html") => "text/html; charset=utf-8",
                Some("js") => "application/javascript; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                _ => "application/octet-stream",
            };
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime)],
                asset.data.into_owned(),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}
