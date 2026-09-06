//! Bounded, site-neutral commands. Credentials are never a research result.
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) const MAX_RESULT_BYTES: usize = 64 * 1024;
pub(crate) const MAX_COMMAND_BYTES: usize = 16 * 1024;
pub(crate) type ResearchResult<T> = Result<T, &'static str>;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SiteManifest {
    pub schema: String,
    pub id: String,
    pub name: String,
    pub entry_url: String,
    pub navigation_origins: Vec<String>,
    pub resource_origins: Vec<String>,
    pub api_origins: Vec<String>,
    pub identity_origins: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResearchCommand {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<SiteManifest>,
}

pub(crate) fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

impl ResearchCommand {
    pub(crate) fn validate(&self) -> ResearchResult<()> {
        if serde_json::to_vec(self)
            .map_err(|_| "invalid_command")?
            .len()
            > MAX_COMMAND_BYTES
        {
            return Err("command_too_large");
        }
        let permitted: &[&str] = match self.kind.as_str() {
            "sites" | "sessions" => &["offset", "limit"],
            "register_site" => &["manifest"],
            "open" => &["site_id"],
            "status" => &["session_id"],
            "pause" | "resume" => &["session_id"],
            "resources" | "requests" => &["session_id", "offset", "limit"],
            "search" => &["session_id", "query", "offset", "limit"],
            "read_resource" => &["session_id", "resource_id", "offset", "limit"],
            "read_request" => &["session_id", "request_id", "offset", "limit"],
            _ => return Err("invalid_command"),
        };
        let value = serde_json::to_value(self).map_err(|_| "invalid_command")?;
        if value
            .as_object()
            .ok_or("invalid_command")?
            .keys()
            .any(|key| key != "kind" && !permitted.contains(&key.as_str()))
        {
            return Err("invalid_command");
        }
        for id in [
            &self.site_id,
            &self.session_id,
            &self.resource_id,
            &self.request_id,
        ]
        .into_iter()
        .flatten()
        {
            if !identifier(id) {
                return Err("invalid_identifier");
            }
        }
        if self.kind == "open" && self.site_id.is_none()
            || matches!(
                self.kind.as_str(),
                "resources"
                    | "requests"
                    | "search"
                    | "read_resource"
                    | "read_request"
                    | "pause"
                    | "resume"
                    | "status"
            ) && self.session_id.is_none()
            || self.kind == "read_resource" && self.resource_id.is_none()
            || self.kind == "read_request" && self.request_id.is_none()
        {
            return Err("missing_argument");
        }
        if self.kind == "search" {
            let query = self.query.as_deref().ok_or("missing_argument")?;
            if query.trim().is_empty() || query.len() > 200 || query.chars().any(char::is_control) {
                return Err("invalid_query");
            }
        }
        let maximum = if self.kind.starts_with("read_") {
            8192
        } else {
            50
        };
        if self.limit.is_some_and(|v| v == 0 || v > maximum)
            || self.offset.is_some_and(|v| v > 9_007_199_254_740_991)
        {
            return Err("invalid_range");
        }
        if self.kind == "register_site" {
            self.manifest
                .as_ref()
                .ok_or("missing_argument")?
                .validate()?;
        }
        Ok(())
    }
}

impl SiteManifest {
    pub(crate) fn validate(&self) -> ResearchResult<()> {
        if self.schema != "yilong.browser-research.site.v1"
            || !identifier(&self.id)
            || self.id.len() > 64
            || self.name.trim().is_empty()
            || self.name.len() > 160
            || self.name.chars().any(char::is_control)
        {
            return Err("invalid_site_manifest");
        }
        let entry = checked_url(&self.entry_url)?;
        if entry.query().is_some()
            || entry.fragment().is_some()
            || self.navigation_origins.is_empty()
        {
            return Err("invalid_site_manifest");
        }
        for origins in [
            &self.navigation_origins,
            &self.resource_origins,
            &self.api_origins,
            &self.identity_origins,
        ] {
            if origins.len() > 16 {
                return Err("invalid_site_manifest");
            }
            let mut seen = std::collections::HashSet::new();
            for origin in origins {
                let parsed = checked_url(origin)?;
                if parsed.origin().ascii_serialization() != *origin || !seen.insert(origin) {
                    return Err("invalid_site_manifest");
                }
            }
        }
        if !self
            .navigation_origins
            .contains(&entry.origin().ascii_serialization())
        {
            return Err("invalid_site_manifest");
        }
        Ok(())
    }
}

fn checked_url(value: &str) -> ResearchResult<reqwest::Url> {
    if value.len() > 2048 || value.chars().any(char::is_control) {
        return Err("invalid_site_manifest");
    }
    let url = reqwest::Url::parse(value).map_err(|_| "invalid_site_manifest")?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.host_str().is_none()
        || url.host_str().is_some_and(|host| host.contains('*'))
        || !(url.scheme() == "https"
            || url.scheme() == "http" && url.host_str() == Some("127.0.0.1"))
    {
        return Err("invalid_site_manifest");
    }
    Ok(url)
}

fn credential_key(key: &str) -> bool {
    let key = key
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase();
    matches!(
        key.as_str(),
        "cookie"
            | "cookies"
            | "setcookie"
            | "authorization"
            | "proxyauthorization"
            | "password"
            | "passwd"
            | "accesstoken"
            | "refreshtoken"
            | "idtoken"
            | "csrftoken"
            | "xsrftoken"
            | "xcsrftoken"
            | "xxsrftoken"
            | "apikey"
            | "secretkey"
            | "sessiontoken"
            | "xmbxapikey"
            | "signature"
            | "sessionkey"
            | "apisecret"
            | "clientsecret"
            | "listenkey"
            | "secret"
            | "credential"
            | "credentials"
    )
}

pub(crate) fn validate_result(value: &Value) -> ResearchResult<()> {
    if serde_json::to_vec(value)
        .map_err(|_| "invalid_result")?
        .len()
        > MAX_RESULT_BYTES
    {
        return Err("result_too_large");
    }
    fn visit(value: &Value, depth: usize, remaining: &mut usize) -> ResearchResult<()> {
        if depth > 24 || *remaining == 0 {
            return Err("invalid_result");
        }
        *remaining -= 1;
        match value {
            Value::Object(fields) => {
                for (key, item) in fields {
                    if credential_key(key) && item.as_str() != Some("[credential_excluded]") {
                        return Err("credentials_forbidden");
                    }
                    visit(item, depth + 1, remaining)?;
                }
            }
            Value::Array(items) => {
                for item in items {
                    visit(item, depth + 1, remaining)?;
                }
            }
            Value::String(text) => {
                if text
                    .trim_start()
                    .to_ascii_lowercase()
                    .starts_with("bearer ")
                {
                    return Err("credentials_forbidden");
                }
                if let Ok(url) = reqwest::Url::parse(text) {
                    if !url.username().is_empty()
                        || url.password().is_some()
                        || url.query_pairs().any(|(key, value)| {
                            credential_key(&key) && value != "[credential_excluded]"
                        })
                    {
                        return Err("credentials_forbidden");
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    visit(value, 0, &mut 8192)
}

pub(crate) fn valid_error(code: &str) -> bool {
    matches!(
        code,
        "operation_failed"
            | "host_unavailable"
            | "invalid_command"
            | "invalid_scope"
            | "session_not_found"
            | "session_expired"
            | "resource_not_found"
            | "request_not_found"
            | "resource_unavailable"
            | "credentials_forbidden"
            | "limit_exceeded"
            | "unsupported"
            | "site_not_found"
            | "navigation_blocked"
            | "result_too_large"
    )
}
