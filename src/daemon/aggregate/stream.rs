use crate::daemon::aggregate::state::AliasMap;
use ccs_proxy::CaptureEvent;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

pub type ProxyEventReceiver = (String, broadcast::Receiver<CaptureEvent>);

#[derive(Debug, Clone, Serialize)]
pub struct TaggedCaptureEvent {
    pub upstream: String,
    pub aliases: Vec<String>,
    #[serde(flatten)]
    pub inner: CaptureEvent,
}

pub async fn event_merger(
    proxy_events: Vec<ProxyEventReceiver>,
    alias_map: Arc<AliasMap>,
    merged_tx: broadcast::Sender<TaggedCaptureEvent>,
) {
    let streams: Vec<_> = proxy_events
        .into_iter()
        .map(|(upstream, rx)| {
            let upstream = upstream.clone();
            BroadcastStream::new(rx)
                .filter_map(move |res| res.ok().map(|ev| (upstream.clone(), ev)))
        })
        .collect();

    let mut merged = futures::stream::select_all(streams);

    while let Some((upstream, event)) = merged.next().await {
        let aliases = alias_map.aliases_for(&upstream);
        let tagged = TaggedCaptureEvent {
            upstream,
            aliases,
            inner: event,
        };
        let _ = merged_tx.send(tagged);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccs_proxy::CaptureEvent;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn merger_tags_events_with_upstream() {
        let (tx_a, _) = broadcast::channel::<CaptureEvent>(16);
        let (tx_b, _) = broadcast::channel::<CaptureEvent>(16);
        let (merged_tx, mut merged_rx) = broadcast::channel::<TaggedCaptureEvent>(64);

        let alias_map = Arc::new(AliasMap::from_entries(vec![(
            "https://a.example.com".to_string(),
            vec!["alias_a".to_string()],
        )]));

        let proxy_events = vec![
            ("https://a.example.com".to_string(), tx_a.subscribe()),
            ("https://b.example.com".to_string(), tx_b.subscribe()),
        ];

        let _merger = tokio::spawn(event_merger(proxy_events, alias_map, merged_tx));

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        tx_a.send(CaptureEvent::RequestStarted {
            session_id: "sess1".to_string(),
            seq: 1,
            started_at: chrono::Utc::now(),
            model: Some("claude-sonnet-4-6".to_string()),
        })
        .unwrap();

        let tagged = tokio::time::timeout(std::time::Duration::from_secs(1), merged_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(tagged.upstream, "https://a.example.com");
        assert_eq!(tagged.aliases, vec!["alias_a"]);
    }

    #[tokio::test]
    async fn merger_handles_unknown_upstream_aliases() {
        let (tx_b, _) = broadcast::channel::<CaptureEvent>(16);
        let (merged_tx, mut merged_rx) = broadcast::channel::<TaggedCaptureEvent>(64);

        let alias_map = Arc::new(AliasMap::from_entries(vec![]));

        let proxy_events = vec![("https://b.example.com".to_string(), tx_b.subscribe())];

        let _merger = tokio::spawn(event_merger(proxy_events, alias_map, merged_tx));

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        tx_b.send(CaptureEvent::RequestStarted {
            session_id: "sess2".to_string(),
            seq: 1,
            started_at: chrono::Utc::now(),
            model: None,
        })
        .unwrap();

        let tagged = tokio::time::timeout(std::time::Duration::from_secs(1), merged_rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(tagged.upstream, "https://b.example.com");
        assert!(tagged.aliases.is_empty());
    }
}
