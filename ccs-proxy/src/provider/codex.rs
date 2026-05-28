//! OpenAI Responses API streaming reassembler.
//!
//! Spec: <https://platform.openai.com/docs/api-reference/responses-streaming>

use crate::capture::Usage;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct CodexResponse {
    pub id: Option<String>,
    pub model: Option<String>,
    pub status: Option<String>,
    pub text_parts: Vec<String>,
    pub usage: Option<Usage>,
}

impl CodexResponse {
    pub fn text_content(&self) -> String {
        self.text_parts.concat()
    }

    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "id": self.id,
            "model": self.model,
            "status": self.status,
            "output_text": self.text_content(),
            "usage": self.usage,
        })
    }
}

pub struct CodexReassembler {
    buffer: Vec<u8>,
    resp: CodexResponse,
    frames_count: u64,
}

impl Default for CodexReassembler {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexReassembler {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            resp: CodexResponse::default(),
            frames_count: 0,
        }
    }

    pub fn feed(&mut self, chunk: &[u8]) {
        self.buffer.extend_from_slice(chunk);
        while let Some(end) = find_double_newline(&self.buffer) {
            let frame_bytes = self.buffer.drain(..end + 2).collect::<Vec<u8>>();
            self.process_frame(&frame_bytes);
        }
    }

    pub fn frames_count(&self) -> u64 {
        self.frames_count
    }

    pub fn finish(mut self) -> Option<CodexResponse> {
        if !self.buffer.is_empty() {
            let leftover = std::mem::take(&mut self.buffer);
            self.process_frame(&leftover);
        }
        if self.frames_count == 0 {
            return None;
        }
        Some(self.resp)
    }

    fn process_frame(&mut self, raw: &[u8]) {
        self.frames_count += 1;
        let mut data_lines: Vec<&[u8]> = Vec::new();
        for line in raw.split(|b| *b == b'\n') {
            let line = strip_cr(line);
            if let Some(rest) = line.strip_prefix(b"data:") {
                let trimmed = trim_ascii_start(rest);
                data_lines.push(trimmed);
            }
        }
        if data_lines.is_empty() {
            return;
        }
        let mut joined = Vec::new();
        for (i, l) in data_lines.iter().enumerate() {
            if i > 0 {
                joined.push(b'\n');
            }
            joined.extend_from_slice(l);
        }
        let Ok(text) = std::str::from_utf8(&joined) else {
            return;
        };
        let Ok(value) = serde_json::from_str::<Value>(text) else {
            return;
        };
        self.apply_event(&value);
    }

    fn apply_event(&mut self, v: &Value) {
        let ty = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
        match ty {
            "response.created" | "response.in_progress" => {
                if let Some(r) = v.get("response") {
                    if let Some(id) = r.get("id").and_then(|x| x.as_str()) {
                        self.resp.id = Some(id.to_string());
                    }
                    if let Some(model) = r.get("model").and_then(|x| x.as_str()) {
                        self.resp.model = Some(model.to_string());
                    }
                    if let Some(status) = r.get("status").and_then(|x| x.as_str()) {
                        self.resp.status = Some(status.to_string());
                    }
                }
            }
            "response.output_text.delta" => {
                if let Some(delta) = v.get("delta").and_then(|x| x.as_str()) {
                    self.resp.text_parts.push(delta.to_string());
                }
            }
            "response.completed" => {
                if let Some(r) = v.get("response") {
                    if let Some(status) = r.get("status").and_then(|x| x.as_str()) {
                        self.resp.status = Some(status.to_string());
                    }
                    if let Some(u) = r.get("usage") {
                        self.resp.usage = Some(parse_usage(u));
                    }
                }
            }
            _ => {}
        }
    }
}

fn parse_usage(v: &Value) -> Usage {
    Usage {
        input_tokens: v.get("input_tokens").and_then(|x| x.as_u64()).unwrap_or(0),
        output_tokens: v.get("output_tokens").and_then(|x| x.as_u64()).unwrap_or(0),
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: v
            .get("cache_read_input_tokens")
            .and_then(|x| x.as_u64())
            .unwrap_or(0),
    }
}

fn find_double_newline(buf: &[u8]) -> Option<usize> {
    // Returns index such that caller `drain(..idx + 2)` consumes the full SSE
    // terminator. For "\n\n" at i, returns i. For "\r\n\r\n" at i, returns
    // i + 2 (so drain(..i + 4) covers all four bytes). Inlined here rather
    // than cross-imported from `super::claude` to avoid polluting that
    // module's public surface with a `#[doc(hidden)]` helper.
    let mut i = 0;
    while i + 1 < buf.len() {
        if buf[i] == b'\n' && buf[i + 1] == b'\n' {
            return Some(i);
        }
        if i + 3 < buf.len() && &buf[i..i + 4] == b"\r\n\r\n" {
            return Some(i + 2);
        }
        i += 1;
    }
    None
}

fn strip_cr(line: &[u8]) -> &[u8] {
    if line.ends_with(b"\r") {
        &line[..line.len() - 1]
    } else {
        line
    }
}

fn trim_ascii_start(s: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < s.len() && (s[i] == b' ' || s[i] == b'\t') {
        i += 1;
    }
    &s[i..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_double_newline_handles_lf_and_crlf() {
        assert_eq!(find_double_newline(b"abc\n\ndef"), Some(3));
        assert_eq!(find_double_newline(b"abc\r\n\r\ndef"), Some(5));
    }
}
