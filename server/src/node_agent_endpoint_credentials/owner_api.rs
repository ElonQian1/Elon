use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use futures::StreamExt;
use reqwest::{redirect::Policy, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::types::{
    EndpointAuthorityBinding, EndpointSecret, ExpectedEndpointCredential, PendingEndpointMutation,
    PendingMutationAction,
};

const MAX_RESPONSE_BYTES: usize = 64 * 1024;

pub(super) fn secure_https_client(timeout: Duration) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .no_proxy()
        .https_only(true)
        .redirect(Policy::none())
        .timeout(timeout)
        .build()
        .context("无法创建 direct-TLS endpoint HTTP client")
}

pub(super) struct OwnerEndpointApi {
    client: reqwest::Client,
    origin: String,
}

impl OwnerEndpointApi {
    pub(super) fn new(origin: &str) -> Result<Self> {
        Ok(Self {
            client: secure_https_client(Duration::from_secs(20))?,
            origin: super::normalize_endpoint_https_origin(origin)?,
        })
    }

    pub(super) async fn execute(
        &self,
        bearer: &str,
        password: &str,
        pending: &PendingEndpointMutation,
    ) -> Result<SecretMutationDelivery> {
        validate_sensitive_input("bearer", bearer, 8_192)?;
        validate_sensitive_input("password", password, 4_096)?;
        pending.validate()?;
        match pending.action {
            PendingMutationAction::Issue => self.issue(bearer, password, pending).await,
            PendingMutationAction::Recover => self.recover(bearer, password, pending).await,
        }
    }

    async fn issue(
        &self,
        bearer: &str,
        password: &str,
        pending: &PendingEndpointMutation,
    ) -> Result<SecretMutationDelivery> {
        let url = format!("{}/api/me/node-endpoint-credentials/issue", self.origin);
        let body = IssueRequest {
            authorization_issuance_request_id: &pending.authorization_issuance_request_id,
            credential_mutation_request_id: &pending.credential_mutation_request_id,
            agent_id: &pending.agent_id,
            install_id: &pending.install_id,
            password,
            confirm_issue: true,
        };
        let response = self
            .client
            .post(url)
            .bearer_auth(bearer)
            .json(&body)
            .send()
            .await
            .context("endpoint credential issue 请求未收到可信响应")?;
        parse_mutation_response(response, &pending.owner_user_id, pending).await
    }

    async fn recover(
        &self,
        bearer: &str,
        password: &str,
        pending: &PendingEndpointMutation,
    ) -> Result<SecretMutationDelivery> {
        let expected = pending
            .expected_credential
            .as_ref()
            .ok_or_else(|| anyhow!("NODE_ENDPOINT_RECOVERY_EXPECTED_CREDENTIAL_REQUIRED"))?;
        let mut url = reqwest::Url::parse(&format!(
            "{}/api/me/node-endpoint-credentials/",
            self.origin
        ))?;
        url.path_segments_mut()
            .map_err(|_| anyhow!("NODE_ENDPOINT_HTTPS_ORIGIN_INVALID"))?
            .push(&pending.agent_id)
            .push("recover");
        let body = RecoverRequest {
            authorization_issuance_request_id: &pending.authorization_issuance_request_id,
            credential_mutation_request_id: &pending.credential_mutation_request_id,
            install_id: &pending.install_id,
            expected_credential: expected,
            password,
            confirm_recovery: true,
        };
        let response = self
            .client
            .post(url)
            .bearer_auth(bearer)
            .json(&body)
            .send()
            .await
            .context("endpoint credential recovery 请求未收到可信响应")?;
        parse_mutation_response(response, &pending.owner_user_id, pending).await
    }
}

pub(super) enum SecretMutationDelivery {
    SecretVisible {
        binding: EndpointAuthorityBinding,
        secret: EndpointSecret,
    },
    ReplayWithoutSecret {
        binding: EndpointAuthorityBinding,
    },
}

#[derive(Serialize)]
struct IssueRequest<'a> {
    authorization_issuance_request_id: &'a str,
    credential_mutation_request_id: &'a str,
    agent_id: &'a str,
    install_id: &'a str,
    password: &'a str,
    confirm_issue: bool,
}

#[derive(Serialize)]
struct RecoverRequest<'a> {
    authorization_issuance_request_id: &'a str,
    credential_mutation_request_id: &'a str,
    install_id: &'a str,
    expected_credential: &'a ExpectedEndpointCredential,
    password: &'a str,
    confirm_recovery: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MutationResponse {
    credential: MutationCredentialResponse,
    consumption_id: String,
    consumption_digest: String,
    replayed: bool,
    result_is_current: bool,
    secret_visible_once: bool,
    endpoint_secret: Option<String>,
    error_code: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MutationCredentialResponse {
    credential_id: String,
    agent_id: String,
    install_id: String,
    credential_revision: u64,
    credential_digest: String,
    status: String,
}

#[derive(Deserialize)]
struct ErrorResponse {
    #[serde(default)]
    error_code: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

async fn parse_mutation_response(
    response: reqwest::Response,
    owner_user_id: &str,
    pending: &PendingEndpointMutation,
) -> Result<SecretMutationDelivery> {
    let status = response.status();
    if status != StatusCode::OK && status != StatusCode::CONFLICT {
        let error: ErrorResponse = read_json_limited(response).await.unwrap_or(ErrorResponse {
            error_code: None,
            error: None,
        });
        let code = error
            .error_code
            .or(error.error)
            .unwrap_or_else(|| "NODE_ENDPOINT_OWNER_API_REJECTED".to_string());
        bail!("endpoint owner API 拒绝请求 ({status}): {code}");
    }
    let mut body: MutationResponse = read_json_limited(response).await?;
    let endpoint_secret = body
        .endpoint_secret
        .take()
        .map(EndpointSecret::from_string)
        .transpose()?;
    if body.consumption_id.is_empty()
        || body.consumption_id != body.consumption_id.trim()
        || body.consumption_id.len() > 160
        || body.consumption_id.chars().any(char::is_control)
        || body.consumption_digest.len() != 64
        || !body
            .consumption_digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        || !body.result_is_current
    {
        bail!("NODE_ENDPOINT_OWNER_API_RESPONSE_INVALID");
    }
    let binding = EndpointAuthorityBinding {
        agent_id: body.credential.agent_id,
        owner_user_id: owner_user_id.to_string(),
        install_id: body.credential.install_id,
        credential_id: body.credential.credential_id,
        credential_revision: body.credential.credential_revision,
        credential_digest: body.credential.credential_digest,
        status: body.credential.status,
    };
    binding.validate()?;
    if binding.agent_id != pending.agent_id
        || binding.owner_user_id != pending.owner_user_id
        || binding.install_id != pending.install_id
        || binding.status != "active"
    {
        bail!("NODE_ENDPOINT_OWNER_API_IDENTITY_MISMATCH");
    }
    match status {
        StatusCode::OK
            if !body.replayed && body.secret_visible_once && body.error_code.is_none() =>
        {
            let secret = endpoint_secret
                .ok_or_else(|| anyhow!("NODE_ENDPOINT_OWNER_API_RESPONSE_INVALID"))?;
            Ok(SecretMutationDelivery::SecretVisible { binding, secret })
        }
        StatusCode::CONFLICT
            if body.replayed
                && !body.secret_visible_once
                && endpoint_secret.is_none()
                && body.error_code.as_deref() == Some("NODE_ENDPOINT_SECRET_NOT_REPLAYABLE") =>
        {
            Ok(SecretMutationDelivery::ReplayWithoutSecret { binding })
        }
        _ => bail!("NODE_ENDPOINT_OWNER_API_RESPONSE_INVALID"),
    }
}

pub(crate) async fn read_json_limited<T: DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
    {
        bail!("NODE_ENDPOINT_HTTPS_RESPONSE_SIZE_INVALID");
    }
    let mut stream = response.bytes_stream();
    let mut bytes = SensitiveResponseBuffer(Vec::new());
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("无法读取 endpoint HTTPS 响应")?;
        let next_len = bytes
            .0
            .len()
            .checked_add(chunk.len())
            .ok_or_else(|| anyhow!("NODE_ENDPOINT_HTTPS_RESPONSE_SIZE_INVALID"))?;
        if next_len > MAX_RESPONSE_BYTES {
            bail!("NODE_ENDPOINT_HTTPS_RESPONSE_SIZE_INVALID");
        }
        bytes.0.extend_from_slice(&chunk);
    }
    if bytes.0.is_empty() {
        bail!("NODE_ENDPOINT_HTTPS_RESPONSE_SIZE_INVALID");
    }
    serde_json::from_slice(&bytes.0).context("NODE_ENDPOINT_HTTPS_RESPONSE_INVALID")
}

struct SensitiveResponseBuffer(Vec<u8>);

impl Drop for SensitiveResponseBuffer {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

fn validate_sensitive_input(name: &str, value: &str, max: usize) -> Result<()> {
    if value.is_empty() || value.len() > max || value.contains(['\r', '\n', '\0']) {
        bail!("NODE_ENDPOINT_{name}_INVALID");
    }
    Ok(())
}
