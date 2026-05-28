//! Split the upstream byte stream so that every chunk is sent to the HTTP
//! client without buffering AND a clone of every chunk is delivered to a
//! background reassembler that produces the final JSON message used by the
//! capture record.

use crate::capture::{CaptureError, ErrorKind};
use crate::provider::{ProviderKind, claude::ClaudeReassembler, codex::CodexReassembler};
use bytes::Bytes;
use futures::StreamExt;
use futures::stream::Stream;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

pub type TapReceiver = mpsc::Receiver<Bytes>;

/// One item of the client-side stream returned by `tee`.
pub type ClientChunk = Result<Bytes, std::io::Error>;

/// Splits an upstream byte stream into:
/// - the returned `Stream` that forwards every chunk to the HTTP client (no
///   buffering — the stream is forwarded as it arrives, preserving TTFT)
/// - a `TapReceiver` that yields a (cheap, `Arc`-backed) clone of every chunk
///   for background reassembly. If the tap channel fills up the chunk is
///   dropped — the client copy is never delayed by the tap.
pub fn tee<S>(upstream: S) -> (impl Stream<Item = ClientChunk>, TapReceiver)
where
    S: Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
{
    let (tap_tx, tap_rx) = mpsc::channel::<Bytes>(64);
    let (out_tx, out_rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(64);
    tokio::spawn(async move {
        let mut up = std::pin::pin!(upstream);
        while let Some(item) = up.next().await {
            match item {
                Ok(chunk) => {
                    // tap is best-effort; never block the client side
                    let _ = tap_tx.try_send(chunk.clone());
                    if out_tx.send(Ok(chunk)).await.is_err() {
                        break;
                    }
                }
                Err(err) => {
                    let io_err = std::io::Error::other(err);
                    let _ = out_tx.send(Err(io_err)).await;
                    break;
                }
            }
        }
        drop(tap_tx);
    });
    (ReceiverStream::new(out_rx), tap_rx)
}

/// Drain the tap stream into the provider-specific reassembler, then return
/// the final reassembled JSON message (when available), the frame count, and
/// an optional partial-capture error.
pub async fn reassemble(
    provider: ProviderKind,
    mut rx: TapReceiver,
) -> (Option<serde_json::Value>, u64, Option<CaptureError>) {
    match provider {
        ProviderKind::Claude => {
            let mut reasm = ClaudeReassembler::new();
            while let Some(chunk) = rx.recv().await {
                reasm.feed(&chunk);
            }
            let count = reasm.frames_count();
            match reasm.finish() {
                Some(msg) => (Some(msg.to_json()), count, None),
                None => (
                    None,
                    count,
                    Some(CaptureError {
                        kind: ErrorKind::ReassembleFailed,
                        message: "no SSE frames parsed".into(),
                    }),
                ),
            }
        }
        ProviderKind::Codex => {
            let mut reasm = CodexReassembler::new();
            while let Some(chunk) = rx.recv().await {
                reasm.feed(&chunk);
            }
            let count = reasm.frames_count();
            match reasm.finish() {
                Some(msg) => (Some(msg.to_json()), count, None),
                None => (
                    None,
                    count,
                    Some(CaptureError {
                        kind: ErrorKind::ReassembleFailed,
                        message: "no SSE frames parsed".into(),
                    }),
                ),
            }
        }
    }
}
