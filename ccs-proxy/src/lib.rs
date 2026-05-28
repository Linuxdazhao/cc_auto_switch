//! ccs-proxy: a local logging reverse-proxy + dashboard that captures the
//! traffic between Claude Code / Codex and their upstream LLM APIs.

pub mod api;
pub mod capture;
mod config;
mod error;
mod handle;
pub mod provider;
pub mod proxy;
mod session;
mod state;
pub mod store;

pub use config::ServeConfig;
pub use error::ServeError;
pub use handle::ProxyHandle;
pub use provider::ProviderKind;
pub use session::SessionId;
pub use state::AppState;

use crate::store::Store;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// Bind both the reverse-proxy and the API listeners on `127.0.0.1`, wire
/// shared [`AppState`], spawn the server task, and return a [`ProxyHandle`]
/// that the caller can use to discover the bound ports and trigger shutdown.
///
/// The configured `proxy_port` / `api_port` of `0` ask the kernel to assign a
/// free port; the actually-bound ports are recorded in both the returned
/// handle and the persisted [`store::SessionMeta`].
#[allow(clippy::too_many_lines)]
pub async fn serve(cfg: ServeConfig) -> Result<ProxyHandle, ServeError> {
    if !cfg!(unix) {
        return Err(ServeError::UnsupportedPlatform(
            "only unix (macOS / Linux) is supported in v1",
        ));
    }

    std::fs::create_dir_all(&cfg.data_dir).map_err(|err| ServeError::DataDir {
        path: cfg.data_dir.clone(),
        source: err,
    })?;

    let store: Arc<dyn Store> = Arc::new(
        store::FsStore::open(cfg.data_dir.clone())
            .map_err(|err| ServeError::Internal(anyhow::Error::from(err)))?,
    );
    let session_id = SessionId::new();

    let proxy_listener = TcpListener::bind(("127.0.0.1", cfg.proxy_port))
        .await
        .map_err(ServeError::BindProxy)?;
    let api_listener = TcpListener::bind(("127.0.0.1", cfg.api_port))
        .await
        .map_err(ServeError::BindApi)?;
    let proxy_addr = proxy_listener.local_addr().map_err(ServeError::BindProxy)?;
    let api_addr = api_listener.local_addr().map_err(ServeError::BindApi)?;

    let meta = store::SessionMeta {
        session_id: session_id.to_string(),
        provider: cfg.provider.as_str().into(),
        upstream: cfg.upstream.to_string(),
        proxy_port: proxy_addr.port(),
        api_port: api_addr.port(),
        started_at: chrono::Utc::now(),
        ended_at: None,
        request_count: 0,
        schema_version: 1,
    };
    if let Err(err) = store.init_session(meta).await {
        tracing::warn!(error = %err, "failed to persist initial session metadata");
    }

    let state = AppState::new(
        store.clone(),
        cfg.provider,
        cfg.upstream.clone(),
        session_id.clone(),
        cfg.redact,
    );

    let proxy_app = proxy::build_proxy_app(state.clone());
    let api_app = api::build_api_app(state);

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let join = spawn_servers(
        proxy_listener,
        api_listener,
        proxy_app,
        api_app,
        shutdown_rx,
        store,
        session_id,
    );

    Ok(ProxyHandle {
        provider: cfg.provider,
        upstream: cfg.upstream,
        proxy_port: proxy_addr.port(),
        api_port: api_addr.port(),
        shutdown_tx: Some(shutdown_tx),
        join: Some(join),
    })
}

fn spawn_servers(
    proxy_listener: TcpListener,
    api_listener: TcpListener,
    proxy_app: axum::Router,
    api_app: axum::Router,
    shutdown_rx: oneshot::Receiver<()>,
    store: Arc<dyn Store>,
    session_id: SessionId,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let proxy_fut = axum::serve(proxy_listener, proxy_app);
        let api_fut = axum::serve(api_listener, api_app);
        tokio::select! {
            res = proxy_fut => {
                if let Err(err) = res {
                    tracing::warn!(error = %err, "proxy server exited");
                }
            }
            res = api_fut => {
                if let Err(err) = res {
                    tracing::warn!(error = %err, "api server exited");
                }
            }
            _ = shutdown_rx => {}
        }
        if let Err(err) = store.finalize_session(session_id.as_str()).await {
            tracing::warn!(error = %err, "failed to finalize session");
        }
    })
}
