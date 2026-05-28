use ccs_proxy::provider::claude::{ClaudeReassembler, ContentBlock};

#[test]
fn reassembles_simple_text_stream() {
    let raw = std::fs::read_to_string("tests/fixtures/claude_stream.sse").unwrap();
    let mut r = ClaudeReassembler::new();
    for chunk in raw.as_bytes().chunks(64) {
        r.feed(chunk);
    }
    let out = r.finish().expect("expected a final message");
    assert_eq!(out.model.as_deref(), Some("claude-sonnet-4-6"));
    assert_eq!(out.text_content(), "Hello world");
    let usage = out.usage.expect("usage");
    assert_eq!(usage.input_tokens, 10);
    assert_eq!(usage.output_tokens, 12);
    assert_eq!(out.stop_reason.as_deref(), Some("end_turn"));
}

#[test]
fn handles_byte_by_byte_feed() {
    let raw = std::fs::read_to_string("tests/fixtures/claude_stream.sse").unwrap();
    let mut r = ClaudeReassembler::new();
    for b in raw.as_bytes() {
        r.feed(&[*b]);
    }
    let out = r.finish().expect("expected a final message");
    assert_eq!(out.text_content(), "Hello world");
}

#[test]
fn reassembles_tool_use_stream() {
    let raw = std::fs::read_to_string("tests/fixtures/claude_tool_use_stream.sse").unwrap();
    let mut r = ClaudeReassembler::new();
    for chunk in raw.as_bytes().chunks(64) {
        r.feed(chunk);
    }
    let out = r.finish().expect("expected a final message");
    assert_eq!(out.stop_reason.as_deref(), Some("tool_use"));
    assert_eq!(out.content_blocks.len(), 1);

    // The single block should be a ToolUse with id=toolu_01, name=get_weather.
    // input._partial should be the concatenation of the two partial_json fragments.
    match &out.content_blocks[0] {
        ContentBlock::ToolUse { id, name, input } => {
            assert_eq!(id, "toolu_01");
            assert_eq!(name, "get_weather");
            let partial = input
                .get("_partial")
                .and_then(|v| v.as_str())
                .expect("input._partial string");
            assert_eq!(partial, r#"{"location":"SF"}"#);
        }
        other => panic!("expected ToolUse block, got: {other:?}"),
    }
}
