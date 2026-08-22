use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

const REGISTRY_SCHEMA: &str = "yilong.private-response-authorization.v1";
const REGISTRY_SOURCE: &str = include_str!("private_response_authorizations.v1.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema: String,
    authorizations: Vec<Authorization>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Authorization {
    authorization_id: String,
    provider_id: String,
    product: String,
    origins: Vec<String>,
    endpoint_path_prefixes: Vec<String>,
    data_classes: Vec<String>,
    persistence: String,
    upload: String,
    consent: String,
    raw_retention_seconds: u64,
    expires_at_ms: u64,
    revoked: bool,
    revocation: String,
}

pub(super) fn allows_rich_kind(provider_id: &str, kind: &str) -> bool {
    parse_registry(REGISTRY_SOURCE)
        .ok()
        .is_some_and(|registry| {
            registry.allows(provider_id, &format!("rich_content.{kind}"), now_ms())
        })
}

fn parse_registry(source: &str) -> Result<Registry, String> {
    let registry: Registry =
        serde_json::from_str(source).map_err(|_| "私有响应生产授权清单格式无效。".to_string())?;
    if registry.schema != REGISTRY_SCHEMA {
        return Err("私有响应生产授权清单版本无效。".to_string());
    }
    if registry.authorizations.len() > 16
        || registry.authorizations.iter().any(|entry| !entry.valid())
    {
        return Err("私有响应生产授权清单包含越界条目。".to_string());
    }
    Ok(registry)
}

impl Registry {
    fn allows(&self, provider_id: &str, data_class: &str, at_ms: u64) -> bool {
        self.authorizations.iter().any(|entry| {
            !entry.revoked
                && entry.provider_id == provider_id
                && entry.expires_at_ms >= at_ms
                && entry.data_classes.iter().any(|value| value == data_class)
        })
    }
}

impl Authorization {
    fn valid(&self) -> bool {
        valid_token(&self.authorization_id, 96)
            && matches!(self.provider_id.as_str(), "chatgpt" | "google-ai-mode")
            && !self.product.trim().is_empty()
            && self.product.chars().count() <= 120
            && !self.origins.is_empty()
            && self.origins.len() <= 8
            && self.origins.iter().all(|value| valid_origin(value))
            && !self.endpoint_path_prefixes.is_empty()
            && self.endpoint_path_prefixes.len() <= 24
            && self
                .endpoint_path_prefixes
                .iter()
                .all(|value| valid_endpoint_prefix(value))
            && !self.data_classes.is_empty()
            && self.data_classes.len() <= 16
            && self
                .data_classes
                .iter()
                .all(|value| valid_data_class(value))
            && self.persistence == "sanitized_ast_only"
            && self.upload == "none"
            && self.consent == "official_web_session"
            && self.raw_retention_seconds == 0
            && self.expires_at_ms > 0
            && !self.revocation.trim().is_empty()
            && self.revocation.chars().count() <= 240
    }
}

fn valid_token(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn valid_origin(value: &str) -> bool {
    value.parse::<tauri::Url>().ok().is_some_and(|url| {
        url.scheme() == "https"
            && !url.cannot_be_a_base()
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().map_or(true, |port| port == 443)
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none()
    })
}

fn valid_endpoint_prefix(value: &str) -> bool {
    value.starts_with('/')
        && !value.starts_with("//")
        && value.len() <= 240
        && !value.contains(['?', '#'])
        && !value.contains("..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-'))
}

fn valid_data_class(value: &str) -> bool {
    value
        .strip_prefix("rich_content.")
        .is_some_and(|kind| valid_token(kind, 48))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(expires_at_ms: u64, revoked: bool) -> String {
        format!(
            r#"{{
              "schema":"{REGISTRY_SCHEMA}",
              "authorizations":[{{
                "authorizationId":"vendor-ticket-2026",
                "providerId":"chatgpt",
                "product":"ChatGPT Web",
                "origins":["https://chatgpt.com/"],
                "endpointPathPrefixes":["/backend-api/conversation"],
                "dataClasses":["rich_content.finance"],
                "persistence":"sanitized_ast_only",
                "upload":"none",
                "consent":"official_web_session",
                "rawRetentionSeconds":0,
                "expiresAtMs":{expires_at_ms},
                "revoked":{revoked},
                "revocation":"删除授权条目并重新发布 Win 客户端"
              }}]
            }}"#
        )
    }

    #[test]
    fn production_registry_denies_private_response_by_default() {
        let registry = parse_registry(REGISTRY_SOURCE).unwrap();
        assert!(!registry.allows("chatgpt", "rich_content.finance", 1));
        assert!(!allows_rich_kind("chatgpt", "finance"));
    }

    #[test]
    fn exact_unexpired_provider_and_data_class_are_required() {
        let registry = parse_registry(&fixture(10_000, false)).unwrap();
        assert!(registry.allows("chatgpt", "rich_content.finance", 9_999));
        assert!(!registry.allows("google-ai-mode", "rich_content.finance", 9_999));
        assert!(!registry.allows("chatgpt", "rich_content.weather", 9_999));
        assert!(!registry.allows("chatgpt", "rich_content.finance", 10_001));
        assert!(!parse_registry(&fixture(10_000, true)).unwrap().allows(
            "chatgpt",
            "rich_content.finance",
            9_999
        ));
    }

    #[test]
    fn raw_persistence_or_credential_shaped_entries_fail_closed() {
        let raw = fixture(10_000, false)
            .replace("sanitized_ast_only", "raw_response")
            .replace("\"upload\":\"none\"", "\"upload\":\"cloud\"");
        assert!(parse_registry(&raw).is_err());

        let unknown = fixture(10_000, false).replace(
            "\"revocation\":",
            "\"authorizationToken\":\"secret\",\"revocation\":",
        );
        assert!(parse_registry(&unknown).is_err());
    }
}
