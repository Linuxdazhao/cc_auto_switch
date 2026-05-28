use ccs_proxy::provider::claude::ClaudeReassembler;

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
