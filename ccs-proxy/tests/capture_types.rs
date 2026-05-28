use ccs_proxy::capture::{CaptureRecord, RequestPart, ResponsePart};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn capture_record_round_trips_through_json() {
    let rec = CaptureRecord {
        seq: 1,
        session_id: "sid".into(),
        request_id: Some("req_xyz".into()),
        started_at: chrono::Utc::now(),
        ended_at: Some(chrono::Utc::now()),
        duration_ms: Some(123),
        ttft_ms: Some(10),
        request: RequestPart {
            method: "POST".into(),
            path: "/v1/messages".into(),
            headers: BTreeMap::new(),
            body: json!({"model": "claude"}),
        },
        response: Some(ResponsePart {
            status: 200,
            headers: BTreeMap::new(),
            body_reassembled: Some(json!({"ok": true})),
            raw_sse_text: None,
            raw_sse_frames_count: 42,
        }),
        usage: None,
        model: Some("claude-sonnet-4-6".into()),
        error: None,
        partial: false,
        schema_version: 1,
    };
    let s = serde_json::to_string(&rec).unwrap();
    let back: CaptureRecord = serde_json::from_str(&s).unwrap();
    assert_eq!(back.seq, 1);
    assert_eq!(back.schema_version, 1);
}
