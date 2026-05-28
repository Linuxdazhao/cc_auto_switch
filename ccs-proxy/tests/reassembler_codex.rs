use ccs_proxy::provider::codex::CodexReassembler;

#[test]
fn reassembles_simple_text_stream() {
    let raw = std::fs::read_to_string("tests/fixtures/codex_stream.sse").unwrap();
    let mut r = CodexReassembler::new();
    for chunk in raw.as_bytes().chunks(64) {
        r.feed(chunk);
    }
    let out = r.finish().expect("response");
    assert_eq!(out.model.as_deref(), Some("gpt-5"));
    assert_eq!(out.text_content(), "Hello world");
    let usage = out.usage.expect("usage");
    assert_eq!(usage.input_tokens, 10);
    assert_eq!(usage.output_tokens, 12);
    assert_eq!(out.status.as_deref(), Some("completed"));
}
