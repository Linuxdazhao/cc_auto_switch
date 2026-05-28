use crate::provider::ProviderKind;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use url::Url;

pub struct ProxyHandle {
    pub provider: ProviderKind,
    pub upstream: Url,
    pub proxy_port: u16,
    pub api_port: u16,
    pub(crate) shutdown_tx: Option<oneshot::Sender<()>>,
    pub(crate) join: Option<JoinHandle<()>>,
}

impl Drop for ProxyHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        // task join is best-effort on drop; ProxyHandle::shutdown().await is preferred
    }
}

impl ProxyHandle {
    pub fn is_finished(&self) -> bool {
        self.join.as_ref().is_some_and(|j| j.is_finished())
    }

    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(join) = self.join.take() {
            let _ = join.await;
        }
    }
}
