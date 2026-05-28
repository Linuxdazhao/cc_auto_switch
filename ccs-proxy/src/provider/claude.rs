//! Anthropic Messages reassembler (server-sent events).
//!
//! Spec: <https://docs.anthropic.com/en/api/messages-streaming>

use crate::capture::Usage;
use serde_json::Value;

#[derive(Debug, Default)]
pub struct ClaudeMessage {
    pub model: Option<String>,
    pub stop_reason: Option<String>,
    pub content_blocks: Vec<ContentBlock>,
    pub usage: Option<Usage>,
}

#[derive(Debug)]
pub enum ContentBlock {
    Text(String),
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

impl ClaudeMessage {
    pub fn text_content(&self) -> String {
        let mut out = String::new();
        for b in &self.content_blocks {
            if let ContentBlock::Text(t) = b {
                out.push_str(t);
            }
        }
        out
    }

    pub fn to_json(&self) -> Value {
        let blocks: Vec<Value> = self
            .content_blocks
            .iter()
            .map(|b| match b {
                ContentBlock::Text(t) => serde_json::json!({"type":"text","text":t}),
                ContentBlock::ToolUse { id, name, input } => serde_json::json!({
                    "type":"tool_use","id":id,"name":name,"input":input
                }),
            })
            .collect();
        serde_json::json!({
            "model": self.model,
            "stop_reason": self.stop_reason,
            "content": blocks,
            "usage": self.usage,
        })
    }
}

pub struct ClaudeReassembler {
    buffer: Vec<u8>,
    msg: ClaudeMessage,
    frames_count: u64,
    saw_message_stop: bool,
}

impl Default for ClaudeReassembler {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeReassembler {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            msg: ClaudeMessage::default(),
            frames_count: 0,
            saw_message_stop: false,
        }
    }

    pub fn feed(&mut self, chunk: &[u8]) {
        self.buffer.extend_from_slice(chunk);
        while let Some(end) = find_double_newline(&self.buffer) {
            let frame_bytes = self.buffer.drain(..end + 2).collect::<Vec<u8>>();
            // SSE frames are blank-line-terminated. drain takes through the \n\n.
            self.process_frame(&frame_bytes);
        }
    }

    pub fn frames_count(&self) -> u64 {
        self.frames_count
    }

    pub fn saw_message_stop(&self) -> bool {
        self.saw_message_stop
    }

    pub fn finish(mut self) -> Option<ClaudeMessage> {
        // Process anything still in buffer (no trailing blank line case).
        if !self.buffer.is_empty() {
            let leftover = std::mem::take(&mut self.buffer);
            self.process_frame(&leftover);
        }
        if self.frames_count == 0 {
            return None;
        }
        Some(self.msg)
    }

    fn process_frame(&mut self, raw: &[u8]) {
        self.frames_count += 1;
        // Each frame has lines like "event: foo" and "data: {...}".
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
        let Some(ty) = v.get("type").and_then(|t| t.as_str()) else {
            return;
        };
        match ty {
            "message_start" => {
                if let Some(m) = v.get("message") {
                    if let Some(model) = m.get("model").and_then(|x| x.as_str()) {
                        self.msg.model = Some(model.to_string());
                    }
                    if let Some(u) = m.get("usage") {
                        self.msg.usage = parse_usage(u);
                    }
                }
            }
            "content_block_start" => {
                if let Some(cb) = v.get("content_block") {
                    let kind = cb.get("type").and_then(|x| x.as_str()).unwrap_or("");
                    match kind {
                        "text" => self
                            .msg
                            .content_blocks
                            .push(ContentBlock::Text(String::new())),
                        "tool_use" => self.msg.content_blocks.push(ContentBlock::ToolUse {
                            id: cb
                                .get("id")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(),
                            name: cb
                                .get("name")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string(),
                            input: cb.get("input").cloned().unwrap_or(Value::Null),
                        }),
                        _ => self
                            .msg
                            .content_blocks
                            .push(ContentBlock::Text(String::new())),
                    }
                }
            }
            "content_block_delta" => {
                if let Some(delta) = v.get("delta") {
                    let delta_type = delta.get("type").and_then(|x| x.as_str()).unwrap_or("");
                    let idx = v.get("index").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
                    if let Some(block) = self.msg.content_blocks.get_mut(idx) {
                        match (block, delta_type) {
                            (ContentBlock::Text(s), "text_delta") => {
                                if let Some(t) = delta.get("text").and_then(|x| x.as_str()) {
                                    s.push_str(t);
                                }
                            }
                            (ContentBlock::ToolUse { input, .. }, "input_json_delta") => {
                                if let Some(partial) =
                                    delta.get("partial_json").and_then(|x| x.as_str())
                                {
                                    // Accumulate raw partial JSON in a string under input,
                                    // serialized as string fragment list. v1 stores last seen.
                                    let key = "_partial".to_string();
                                    if let Value::Null = input {
                                        *input = Value::Object(Default::default());
                                    }
                                    if let Value::Object(m) = input {
                                        let cur = m
                                            .entry(key)
                                            .or_insert_with(|| Value::String(String::new()));
                                        if let Value::String(s) = cur {
                                            s.push_str(partial);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            "message_delta" => {
                if let Some(d) = v.get("delta")
                    && let Some(sr) = d.get("stop_reason").and_then(|x| x.as_str())
                {
                    self.msg.stop_reason = Some(sr.to_string());
                }
                if let Some(u) = v.get("usage") {
                    if let Some(existing) = self.msg.usage.as_mut() {
                        if let Some(ot) = u.get("output_tokens").and_then(|x| x.as_u64()) {
                            existing.output_tokens = ot;
                        }
                    } else {
                        self.msg.usage = parse_usage(u);
                    }
                }
            }
            "message_stop" => {
                self.saw_message_stop = true;
            }
            _ => {}
        }
    }
}

fn parse_usage(v: &Value) -> Option<Usage> {
    let mut u = Usage::default();
    if let Some(x) = v.get("input_tokens").and_then(|x| x.as_u64()) {
        u.input_tokens = x;
    }
    if let Some(x) = v.get("output_tokens").and_then(|x| x.as_u64()) {
        u.output_tokens = x;
    }
    if let Some(x) = v
        .get("cache_creation_input_tokens")
        .and_then(|x| x.as_u64())
    {
        u.cache_creation_input_tokens = x;
    }
    if let Some(x) = v.get("cache_read_input_tokens").and_then(|x| x.as_u64()) {
        u.cache_read_input_tokens = x;
    }
    Some(u)
}

fn find_double_newline(buf: &[u8]) -> Option<usize> {
    // returns index such that caller `drain(..idx + 2)` consumes the full
    // terminator. For "\n\n" at i, returns i. For "\r\n\r\n" at i, returns
    // i + 2 (so drain(..i + 4) covers all four bytes).
    let mut i = 0;
    while i + 1 < buf.len() {
        if buf[i] == b'\n' && buf[i + 1] == b'\n' {
            return Some(i);
        }
        if i + 3 < buf.len() && &buf[i..i + 4] == b"\r\n\r\n" {
            // BUG FIX: was Some(i + 1) in plan template — caller drains
            // ..end+2, so we must return i+2 to consume all 4 terminator
            // bytes; the previous value left a stray '\n' in the buffer.
            return Some(i + 2);
        }
        i += 1;
    }
    None
}

fn strip_cr(line: &[u8]) -> &[u8] {
    // BUG FIX: plan template had a dead first branch (bound `rest` and
    // immediately discarded it). Removed.
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
