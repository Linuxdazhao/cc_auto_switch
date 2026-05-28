use ccs_proxy::{ProviderKind, ServeConfig};
use tempfile::TempDir;
use url::Url;

#[tokio::test]
async fn serve_with_api_server_false_returns_none_api_port() {
    let tmp = TempDir::new().unwrap();
    let mut cfg = ServeConfig::new(
        ProviderKind::Claude,
        Url::parse("https://api.anthropic.com").unwrap(),
        tmp.path().to_path_buf(),
    );
    cfg.api_server = false;

    let handle = ccs_proxy::serve(cfg).await.unwrap();
    assert!(
        handle.api_port.is_none(),
        "api_port should be None when api_server=false"
    );
    assert!(handle.proxy_port > 0);
    handle.shutdown().await;
}
