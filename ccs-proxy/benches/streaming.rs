//! Criterion benchmark: round-trip TTFT through the proxy to a wiremock
//! upstream that streams a minimal SSE response. Measures how long a POST
//! to `/v1/messages` plus reading the body takes via `serve()`.

use ccs_proxy::{ProviderKind, ServeConfig, serve};
use criterion::{Criterion, criterion_group, criterion_main};
use std::time::Duration;
use tempfile::tempdir;
use url::Url;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, ResponseTemplate};

const SSE_BODY: &str = concat!(
    "event: message_start\n",
    "data: {\"type\":\"message_start\",\"message\":{\"id\":\"x\",\"type\":\"message\",",
    "\"role\":\"assistant\",\"model\":\"m\",\"content\":[],\"stop_reason\":null,",
    "\"stop_sequence\":null,\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n\n",
    "event: message_stop\n",
    "data: {\"type\":\"message_stop\"}\n\n",
);

fn bench_ttft(crit: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let (_mock_guard, proxy_port, _handle, _dir) = rt.block_on(async {
        let mock = MockServer::start().await;
        Mock::given(any())
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(SSE_BODY),
            )
            .mount(&mock)
            .await;
        let dir = tempdir().expect("tempdir");
        let cfg = ServeConfig::new(
            ProviderKind::Claude,
            Url::parse(&mock.uri()).expect("parse mock uri"),
            dir.path().to_path_buf(),
        );
        let handle = serve(cfg).await.expect("serve");
        let port = handle.proxy_port;
        (mock, port, handle, dir)
    });

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{proxy_port}/v1/messages");
    crit.bench_function("ttft_via_proxy", |bencher| {
        bencher.to_async(&rt).iter(|| async {
            let resp = client
                .post(&url)
                .body(r#"{"model":"m","messages":[]}"#)
                .send()
                .await
                .expect("send");
            let _ = resp.bytes().await.expect("read body");
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = bench_ttft
}
criterion_main!(benches);
