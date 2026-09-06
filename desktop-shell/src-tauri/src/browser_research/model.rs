use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const RESULT_SCHEMA: &str = "yilong.browser-research.result.v1";
pub const BODY_LIMIT: usize = 2 * 1024 * 1024;
pub const SESSION_BYTES: u64 = 256 * 1024 * 1024;
pub const RESOURCE_LIMIT: usize = 512;
pub const SESSION_DURATION: u64 = 60 * 60 * 1000;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiteManifest {
    pub schema: String,
    pub id: String,
    pub name: String,
    pub entry_url: String,
    pub navigation_origins: Vec<String>,
    pub resource_origins: Vec<String>,
    pub api_origins: Vec<String>,
    pub identity_origins: Vec<String>,
}

impl SiteManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "yilong.browser-research.site.v1"
            || !identifier(&self.id)
            || self.name.is_empty()
            || self.name.len() > 160
            || self.entry_url.len() > 2048
            || self.navigation_origins.is_empty()
        {
            return Err("invalid_site_manifest".into());
        }
        for group in [
            &self.navigation_origins,
            &self.resource_origins,
            &self.api_origins,
            &self.identity_origins,
        ] {
            if group.len() > 16
                || group.iter().any(|s| origin(s).as_ref() != Some(s))
                || group.iter().collect::<std::collections::HashSet<_>>().len() != group.len()
            {
                return Err("invalid_site_origin".into());
            }
        }
        let entry = tauri::Url::parse(&self.entry_url).map_err(|_| "invalid_site_entry")?;
        if entry.query().is_some()
            || entry.fragment().is_some()
            || !self.allows(&self.entry_url, "navigation")
            || self.identity(&self.entry_url)
        {
            return Err("invalid_site_entry".into());
        }
        Ok(())
    }
    pub fn identity(&self, url: &str) -> bool {
        origin(url).is_some_and(|o| self.identity_origins.contains(&o))
    }
    pub fn allows(&self, url: &str, kind: &str) -> bool {
        let Some(o) = origin(url) else {
            return false;
        };
        if self.identity_origins.contains(&o) {
            return false;
        }
        match kind {
            "navigation" => self.navigation_origins.contains(&o),
            "api" => self.api_origins.contains(&o),
            _ => self.resource_origins.contains(&o) || self.navigation_origins.contains(&o),
        }
    }
    pub fn fingerprint(&self) -> String {
        hash(&serde_json::to_vec(self).unwrap_or_default())
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchCommand {
    pub kind: String,
    pub site_id: Option<String>,
    pub session_id: Option<String>,
    pub resource_id: Option<String>,
    pub request_id: Option<String>,
    pub query: Option<String>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    pub manifest: Option<SiteManifest>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub url: String,
    pub resource_type: String,
    pub mime: String,
    pub size_bytes: usize,
    pub sha256: String,
    pub generation: u64,
    pub captured_at_ms: u64,
    pub truncated: bool,
    pub redacted: bool,
    pub script_id: Option<String>,
    pub host_request_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request {
    pub id: String,
    pub url: String,
    pub method: String,
    pub status: Option<u16>,
    pub generation: u64,
    pub request_resource_id: Option<String>,
    pub response_resource_id: Option<String>,
    pub initiator: Option<Value>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    pub schema: String,
    pub id: String,
    pub project_key: String,
    pub owner_hash: String,
    pub site: SiteManifest,
    pub site_fingerprint: String,
    pub active: bool,
    pub generation: u64,
    pub expires_at_ms: u64,
    pub phase: String,
    pub bytes: u64,
    pub resources: Vec<Resource>,
    pub requests: Vec<Request>,
    pub gaps: Vec<String>,
}

impl Session {
    pub fn summary(&self) -> Value {
        serde_json::json!({"id":self.id,"site_id":self.site.id,
            "active":self.active && now_ms()<self.expires_at_ms,"generation":self.generation,
            "expires_at_ms":self.expires_at_ms,"phase":self.phase,
            "resource_count":self.resources.len(),"request_count":self.requests.len(),
            "gaps":self.gaps,"trading_enabled":false})
    }
    pub fn gap(&mut self, code: &str) {
        let safe: String = code
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .take(80)
            .collect();
        if self.gaps.len() < 20 && !self.gaps.contains(&safe) {
            self.gaps.push(safe);
        }
    }
}

pub fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
pub fn digest_id(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn hash(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_millis() as u64)
        .unwrap_or_default()
}
pub fn origin(value: &str) -> Option<String> {
    let url = tauri::Url::parse(value).ok()?;
    let local = url.host_str() == Some("127.0.0.1");
    (url.username().is_empty()
        && url.password().is_none()
        && url.host_str().is_some_and(|h| !h.contains('*'))
        && (url.scheme() == "https" || (url.scheme() == "http" && local)))
        .then(|| url.origin().ascii_serialization())
}

pub fn defaults() -> Vec<SiteManifest> {
    // Site-specific facts are data; new sites are registered without recompiling this domain.
    serde_json::from_str::<Vec<SiteManifest>>(include_str!("../../browser-research/sites.json"))
        .unwrap_or_default()
        .into_iter()
        .filter(|site| site.validate().is_ok())
        .collect()
}
