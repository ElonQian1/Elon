//! HTTP response normalization for external app context fetches.

use serde_json::{json, Value};

use crate::external_app_context_contract::{fb2_match_context, fb2_pack_context};

pub(crate) async fn fb2_pack_response_to_context(
    app_id: &str,
    external_group_id: &str,
    response: reqwest::Response,
) -> Value {
    let status = response.status();
    let text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            return json!({
                "app_id": app_id,
                "group": external_group_id,
                "status": "unavailable",
                "error": compact_error(&error.to_string())
            });
        }
    };
    if !status.is_success() {
        return json!({
            "app_id": app_id,
            "group": external_group_id,
            "status": "unavailable",
            "status_code": status.as_u16(),
            "error": truncate_chars(&text, 300)
        });
    }

    let parsed = match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(error) => {
            return json!({
                "app_id": app_id,
                "group": external_group_id,
                "status": "unavailable",
                "error": format!("fb2 context pack JSON 解析失败：{}", compact_error(&error.to_string()))
            });
        }
    };
    if parsed["success"].as_bool() != Some(true) {
        return json!({
            "app_id": app_id,
            "group": external_group_id,
            "status": "unavailable",
            "error": parsed["error"].as_str().unwrap_or("fb2 返回失败状态")
        });
    }

    let data = parsed.get("data").cloned().unwrap_or_else(|| json!({}));
    fb2_pack_context(app_id, external_group_id, data)
}

pub(crate) async fn fb2_response_to_context(
    app_id: &str,
    external_group_id: &str,
    response: reqwest::Response,
) -> Value {
    let status = response.status();
    let text = match response.text().await {
        Ok(text) => text,
        Err(error) => {
            return json!({
                "app_id": app_id,
                "group": external_group_id,
                "status": "unavailable",
                "error": compact_error(&error.to_string())
            });
        }
    };
    if !status.is_success() {
        return json!({
            "app_id": app_id,
            "group": external_group_id,
            "status": "unavailable",
            "status_code": status.as_u16(),
            "error": truncate_chars(&text, 300)
        });
    }

    let parsed = match serde_json::from_str::<Value>(&text) {
        Ok(value) => value,
        Err(error) => {
            return json!({
                "app_id": app_id,
                "group": external_group_id,
                "status": "unavailable",
                "error": format!("fb2 JSON 解析失败：{}", compact_error(&error.to_string()))
            });
        }
    };
    if parsed["success"].as_bool() != Some(true) {
        return json!({
            "app_id": app_id,
            "group": external_group_id,
            "status": "unavailable",
            "error": parsed["error"].as_str().unwrap_or("fb2 返回失败状态")
        });
    }

    let data = parsed.get("data").cloned().unwrap_or_else(|| json!({}));
    fb2_match_context(app_id, external_group_id, data)
}

pub(crate) fn compact_error(error: &str) -> String {
    truncate_chars(error, 220)
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut out = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn truncates_long_errors() {
        let text = "a".repeat(300);
        let truncated = truncate_chars(&text, 20);
        assert_eq!(truncated, "aaaaaaaaaaaaaaaaaaaa...");
    }

    #[tokio::test]
    async fn pack_response_http_error_is_unavailable() {
        let response = test_response(500, &"service failed ".repeat(40)).await;
        let context = fb2_pack_response_to_context("fb2", "official", response).await;

        assert_eq!(context["status"], "unavailable");
        assert_eq!(context["status_code"], 500);
        assert!(context["error"].as_str().unwrap().chars().count() <= 303);
    }

    #[tokio::test]
    async fn pack_response_invalid_json_is_unavailable() {
        let response = test_response(200, "{not-json").await;
        let context = fb2_pack_response_to_context("fb2", "official", response).await;

        assert_eq!(context["status"], "unavailable");
        assert!(context["error"]
            .as_str()
            .unwrap()
            .contains("context pack JSON 解析失败"));
    }

    #[tokio::test]
    async fn pack_response_success_false_is_unavailable() {
        let response = test_response(200, r#"{"success":false,"error":"context blocked"}"#).await;
        let context = fb2_pack_response_to_context("fb2", "official", response).await;

        assert_eq!(context["status"], "unavailable");
        assert_eq!(context["error"], "context blocked");
    }

    #[tokio::test]
    async fn match_response_empty_data_keeps_empty_quality_warning() {
        let response = test_response(
            200,
            r#"{"success":true,"data":{"generated_at":"2026-06-22T12:00:00+08:00","matches":[]}}"#,
        )
        .await;
        let context = fb2_response_to_context("fb2", "official", response).await;

        assert_eq!(context["status"], "ready");
        assert_eq!(context["count"], 0);
        assert!(context["context_quality"]["warnings"]
            .as_array()
            .unwrap()
            .contains(&json!("empty_matches")));
    }

    #[tokio::test]
    async fn pack_response_promotes_too_large_budget_warning() {
        let response = test_response(
            200,
            r#"{"success":true,"data":{"generated_at":"2026-06-22T12:00:00+08:00","context_pack_version":"fb2-chat-pack-v1","context_pack":"<fb2_context_pack>large</fb2_context_pack>","matches":[{"id":"match-1"}],"tool_contract":{"tools":[{"name":"get_match_detail"}]},"metrics":{"budget_status":"too_large"}}}"#,
        )
        .await;
        let context = fb2_pack_response_to_context("fb2", "official", response).await;

        assert_eq!(context["status"], "ready");
        assert!(context["context_quality"]["warnings"]
            .as_array()
            .unwrap()
            .contains(&json!("fb2_budget_too_large")));
    }

    async fn test_response(status_code: u16, body: &str) -> reqwest::Response {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind test listener");
        let addr = listener.local_addr().expect("local addr");
        let body = body.to_string();

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("test request");
            let mut request = Vec::new();
            let mut chunk = [0_u8; 512];
            loop {
                let read = stream.read(&mut chunk).await.expect("read request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }

            let reason = match status_code {
                200 => "OK",
                500 => "Internal Server Error",
                _ => "Test Status",
            };
            let response = format!(
                "HTTP/1.1 {status_code} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.as_bytes().len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        let response = reqwest::Client::new()
            .get(format!("http://{addr}/context"))
            .send()
            .await
            .expect("client response");
        server.await.expect("server task");
        response
    }
}
