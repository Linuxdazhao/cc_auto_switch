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
        let h = BTreeMap::from([(
            "content-type".to_string(),
            "application/json".to_string(),
        )]);
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
}
