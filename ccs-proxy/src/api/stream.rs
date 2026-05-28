use crate::AppState;
use crate::capture::CaptureEvent;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use std::convert::Infallible;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

pub async fn stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.events.subscribe();
    let sse_stream = BroadcastStream::new(rx).filter_map(|res| match res {
        Ok(ev) => Some(Ok(event_to_sse(ev))),
        Err(_lagged) => None, // dropped events: skip silently
    });
    Sse::new(sse_stream).keep_alive(KeepAlive::default())
}

fn event_to_sse(ev: CaptureEvent) -> Event {
    let (name, data) = match &ev {
        CaptureEvent::RequestStarted { .. } => (
            "request_started",
            serde_json::to_string(&ev).unwrap_or_default(),
        ),
        CaptureEvent::RequestCompleted { .. } => (
            "request_completed",
            serde_json::to_string(&ev).unwrap_or_default(),
        ),
    };
    Event::default().event(name).data(data)
}
