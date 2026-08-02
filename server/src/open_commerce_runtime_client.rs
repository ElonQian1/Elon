//! Signed HTTP client for an approved merchant runtime binding.

use anyhow::{anyhow, Error as AnyhowError, Result};
use hmac::{Hmac, Mac};
use reqwest::StatusCode;
use serde_json::Value;
use sha2::Sha256;
use std::fmt;
use std::time::Duration;

use crate::open_commerce_runtime_model::{MerchantRuntimeEnvelope, OpenCommerceRuntimeBinding};

pub(crate) async fn invoke_runtime(
    binding: &OpenCommerceRuntimeBinding,
    envelope: &MerchantRuntimeEnvelope,
) -> std::result::Result<Value, RuntimeCallError> {
    let secret =
        crate::open_commerce_runtime_security::resolve_runtime_secret(&binding.credential_ref)
            .map_err(RuntimeCallError::infrastructure)?;
    let body = serde_json::to_vec(envelope).map_err(RuntimeCallError::infrastructure)?;
    let timestamp = envelope.issued_at_unix.to_string();
    let signature = sign(&secret, &timestamp, &body).map_err(RuntimeCallError::infrastructure)?;
    let endpoint = format!("{}/commerce/v1/invoke", binding.endpoint_base_url);
    let response = reqwest::Client::builder()
        .timeout(Duration::from_millis(binding.timeout_ms as u64))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(RuntimeCallError::infrastructure)?
        .post(endpoint)
        .header("content-type", "application/json")
        .header("x-yilong-runtime-key-id", &binding.credential_ref)
        .header("x-yilong-runtime-timestamp", &timestamp)
        .header("x-yilong-runtime-signature", format!("v1={signature}"))
        .body(body)
        .send()
        .await
        .map_err(|error| {
            RuntimeCallError::infrastructure(anyhow!("商户运行服务不可达: {error}"))
        })?;
    let status = response.status();
    let bytes = response.bytes().await.map_err(|error| {
        RuntimeCallError::infrastructure(anyhow!("读取商户运行响应失败: {error}"))
    })?;
    if bytes.len() > 512 * 1024 {
        return Err(RuntimeCallError::infrastructure(anyhow!(
            "商户运行响应超过 512 KiB 限制"
        )));
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|error| {
        RuntimeCallError::infrastructure(anyhow!("商户运行响应不是有效 JSON: {error}"))
    })?;
    if !status.is_success() {
        let code = value
            .get("error_code")
            .and_then(Value::as_str)
            .unwrap_or("runtime_rejected");
        return Err(RuntimeCallError::rejected(status, code));
    }
    if value.get("schema").and_then(Value::as_str) != Some("merchant_runtime.result.v1")
        || value.get("invocation_id").and_then(Value::as_str)
            != Some(envelope.invocation_id.as_str())
        || value.get("capability_key").and_then(Value::as_str)
            != Some(envelope.capability_key.as_str())
    {
        return Err(RuntimeCallError::infrastructure(anyhow!(
            "商户运行响应身份不匹配"
        )));
    }
    let result = value
        .get("result")
        .cloned()
        .ok_or_else(|| RuntimeCallError::infrastructure(anyhow!("商户运行响应缺少 result")))?;
    crate::open_commerce_merchant_evidence_model::validate_optional_business_receipt(&result)
        .map_err(RuntimeCallError::infrastructure)?;
    Ok(result)
}

#[derive(Debug)]
pub(crate) struct RuntimeCallError {
    message: String,
    degrades_binding: bool,
}

impl RuntimeCallError {
    fn infrastructure(error: impl Into<AnyhowError>) -> Self {
        Self {
            message: error.into().to_string(),
            degrades_binding: true,
        }
    }

    fn rejected(status: StatusCode, code: &str) -> Self {
        Self {
            message: format!("商户运行调用失败: {code}"),
            degrades_binding: status.is_server_error()
                || matches!(
                    status,
                    StatusCode::UNAUTHORIZED
                        | StatusCode::FORBIDDEN
                        | StatusCode::REQUEST_TIMEOUT
                        | StatusCode::TOO_MANY_REQUESTS
                ),
        }
    }

    pub(crate) fn degrades_binding(&self) -> bool {
        self.degrades_binding
    }
}

impl fmt::Display for RuntimeCallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RuntimeCallError {}

fn sign(secret: &str, timestamp: &str, body: &[u8]) -> Result<String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| anyhow!("商户运行签名密钥无效"))?;
    mac.update(timestamp.as_bytes());
    mac.update(b".");
    mac.update(body);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_rejections_do_not_degrade_a_healthy_binding() {
        assert!(
            !RuntimeCallError::rejected(StatusCode::CONFLICT, "inventory_shortage")
                .degrades_binding()
        );
        assert!(
            RuntimeCallError::rejected(StatusCode::SERVICE_UNAVAILABLE, "runtime_disabled")
                .degrades_binding()
        );
    }
}

#[cfg(test)]
pub(crate) fn test_signature(secret: &str, timestamp: &str, body: &[u8]) -> String {
    sign(secret, timestamp, body).unwrap()
}
