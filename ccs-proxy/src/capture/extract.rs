//! Pull out structured metadata (request id, model, usage) from raw
//! request/response shapes.

use serde_json::Value;
use std::collections::BTreeMap;

pub fn extract_request_id(response_headers: &BTreeMap<String, String>) -> Option<String> {
    const CANDIDATES: &[&str] = &[
        "anthropic-request-id",
        "x-request-id",
        "request-id",
        "openai-request-id",
    ];
    for name in CANDIDATES {
        for (k, v) in response_headers.iter() {
            if k.eq_ignore_ascii_case(name) && !v.is_empty() {
                return Some(v.clone());
            }
        }
    }
    None
}

pub fn extract_model_from_request_body(body: &Value) -> Option<String> {
    body.get("model")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Extract the Claude Code working-directory marker from a request body's
/// `system` field. Looks for a line starting with `Primary working directory:`
/// (case-sensitive, anchored to line start) and returns the trimmed path.
/// Handles both `system: "..."` (string) and
/// `system: [{"type":"text","text":"..."}]` (block list) shapes.
pub fn extract_cwd(body: &Value) -> Option<String> {
    let system = body.get("system")?;
    if let Some(s) = system.as_str() {
        return scan_system_text(s);
    }
    if let Some(arr) = system.as_array() {
        for block in arr {
            if let Some(text) = block.get("text").and_then(|v| v.as_str())
                && let Some(found) = scan_system_text(text)
            {
                return Some(found);
            }
        }
    }
    None
}

fn scan_system_text(text: &str) -> Option<String> {
    const MARKER: &str = "Primary working directory:";
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix(MARKER) {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn picks_anthropic_request_id() {
        let h = BTreeMap::from([
            ("Anthropic-Request-Id".to_string(), "req_01H8".to_string()),
            ("content-type".to_string(), "text/event-stream".to_string()),
        ]);
        assert_eq!(extract_request_id(&h), Some("req_01H8".into()));
    }

    #[test]
    fn picks_x_request_id_when_anthropic_missing() {
        let h = BTreeMap::from([("X-Request-Id".to_string(), "abc".to_string())]);
        assert_eq!(extract_request_id(&h), Some("abc".into()));
    }

    #[test]
    fn returns_none_when_no_id_header() {
        let h = BTreeMap::from([("content-type".to_string(), "application/json".to_string())]);
        assert_eq!(extract_request_id(&h), None);
    }

    #[test]
    fn extracts_model_from_body() {
        let body = json!({"model": "claude-sonnet-4-6", "messages": []});
        assert_eq!(
            extract_model_from_request_body(&body),
            Some("claude-sonnet-4-6".into())
        );
    }

    #[test]
    fn no_model_when_field_absent() {
        let body = json!({"messages": []});
        assert_eq!(extract_model_from_request_body(&body), None);
    }

    #[test]
    fn cwd_from_string_system() {
        let body = json!({
            "system": "You are Claude Code.\nPrimary working directory: /Users/me/proj\nMore text.",
        });
        assert_eq!(extract_cwd(&body), Some("/Users/me/proj".into()));
    }

    #[test]
    fn cwd_from_block_list_system() {
        let body = json!({
            "system": [
                {"type": "text", "text": "header"},
                {"type": "text", "text": "intro\nPrimary working directory: /tmp/x y z\ntail"},
            ],
        });
        assert_eq!(extract_cwd(&body), Some("/tmp/x y z".into()));
    }

    #[test]
    fn cwd_ignores_prose_mention() {
        let body = json!({
            "system": "Consider the user's working directory when answering questions.",
        });
        assert_eq!(extract_cwd(&body), None);
    }

    #[test]
    fn cwd_returns_none_when_no_system() {
        let body = json!({"messages": []});
        assert_eq!(extract_cwd(&body), None);
    }

    #[test]
    fn cwd_takes_first_match_only() {
        let body = json!({
            "system": "Primary working directory: /a\nPrimary working directory: /b",
        });
        assert_eq!(extract_cwd(&body), Some("/a".into()));
    }
}
