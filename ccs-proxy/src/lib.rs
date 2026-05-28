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
