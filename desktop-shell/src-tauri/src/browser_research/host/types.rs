use serde::Serialize;
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};

pub(crate) type HostSink = Arc<dyn Fn(HostEvent) + Send + Sync>;

#[derive(Clone)]
pub(crate) struct HostConfig {
    pub label: String,
    pub start_url: String,
    /// Trusted core-derived directory, never supplied by a page or MCP path argument.
    pub profile_dir: PathBuf,
    pub navigation_origins: Vec<String>,
    pub resource_origins: Vec<String>,
    pub api_origins: Vec<String>,
    pub identity_origins: Vec<String>,
    pub max_body_bytes: usize,
    pub expires_at_ms: u64,
}

/// Internal untrusted website material. Core MUST apply its credential policy before storage/output.
#[derive(Clone, Serialize)]
pub(crate) struct HostEvent {
    pub generation: u64,
    pub kind: String,
    pub url: String,
    pub method: Option<String>,
    pub status: Option<u16>,
    pub resource_type: Option<String>,
    pub request_id: Option<String>,
    pub script_id: Option<String>,
    pub request_body: Option<String>,
    pub body: Option<String>,
    pub mime: Option<String>,
    pub initiator: Option<Value>,
    pub truncated: bool,
    pub error_code: Option<String>,
}

impl HostEvent {
    pub(super) fn new(generation: u64, kind: &str, url: &str) -> Self {
        Self {
            generation,
            kind: kind.into(),
            url: url.into(),
            method: None,
            status: None,
            resource_type: None,
            request_id: None,
            script_id: None,
            request_body: None,
            body: None,
            mime: None,
            initiator: None,
            truncated: false,
            error_code: None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct HostHandle {
    pub label: String,
    pub(super) control: Arc<Control>,
}

pub(super) struct Control {
    pub active: AtomicBool,
    pub generation: AtomicU64,
    pub closed: AtomicBool,
    pub expires_at_ms: u64,
    pub sink: HostSink,
}

impl HostHandle {
    pub(crate) fn generation(&self) -> u64 {
        self.control.generation.load(Ordering::SeqCst)
    }
    pub(crate) fn active(&self) -> bool {
        self.control.active.load(Ordering::SeqCst)
            && !self.control.closed.load(Ordering::SeqCst)
            && now_ms() < self.control.expires_at_ms
    }
    pub(super) fn pause(&self) {
        self.control.active.store(false, Ordering::SeqCst);
        let generation = self.control.generation.fetch_add(1, Ordering::SeqCst) + 1;
        (self.control.sink)(HostEvent::new(generation, "paused", ""));
    }
    pub(super) fn accepts(&self, generation: u64) -> bool {
        self.active() && self.generation() == generation
    }
}

impl HostConfig {
    pub(super) fn validate(&self) -> Result<(), String> {
        let valid_label = self.label.starts_with("browser-research-")
            && self.label.len() <= 96
            && self
                .label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-');
        let valid_origins = [
            &self.navigation_origins,
            &self.resource_origins,
            &self.api_origins,
            &self.identity_origins,
        ]
        .into_iter()
        .all(|group| {
            group.len() <= 16
                && group
                    .iter()
                    .all(|s| origin(s).as_deref() == Some(s.as_str()))
        });
        if !valid_label
            || !self.profile_dir.is_absolute()
            || !valid_origins
            || !self.allows_navigation(&self.start_url)
            || self.max_body_bytes == 0
            || self.max_body_bytes > 2 * 1024 * 1024
            || self.expires_at_ms <= now_ms()
        {
            return Err("browser_research_host_config_invalid".into());
        }
        Ok(())
    }
    pub(super) fn allows_navigation(&self, url: &str) -> bool {
        origin(url).is_some_and(|o| {
            self.navigation_origins.contains(&o) || self.identity_origins.contains(&o)
        })
    }
    pub(super) fn allows_document(&self, url: &str) -> bool {
        !sensitive_url(url)
            && origin(url).is_some_and(|o| {
                self.navigation_origins.contains(&o) && !self.identity_origins.contains(&o)
            })
    }
    pub(super) fn allows_resource(&self, url: &str) -> bool {
        !sensitive_url(url)
            && origin(url).is_some_and(|o| {
                !self.identity_origins.contains(&o)
                    && (self.resource_origins.contains(&o) || self.navigation_origins.contains(&o))
            })
    }
    pub(super) fn allows_api(&self, url: &str) -> bool {
        !sensitive_url(url)
            && origin(url).is_some_and(|o| {
                self.api_origins.contains(&o) && !self.identity_origins.contains(&o)
            })
    }
}

fn sensitive_url(value: &str) -> bool {
    const SENSITIVE: &[&str] = &[
        "login",
        "logout",
        "signin",
        "signup",
        "auth",
        "oauth",
        "oauth2",
        "authorize",
        "authorization",
        "authentication",
        "token",
        "accesstoken",
        "refreshtoken",
        "idtoken",
        "session",
        "sessions",
        "sessiontoken",
        "credential",
        "credentials",
        "password",
        "otp",
        "mfa",
        "2fa",
        "apikey",
        "apisecret",
        "csrf",
        "captcha",
    ];
    let Ok(url) = tauri::Url::parse(value) else {
        return true;
    };
    let path_sensitive = url.path_segments().into_iter().flatten().any(|part| {
        let lower = part.to_ascii_lowercase().replace(['-', '_'], "");
        // Percent-encoded path components are ambiguous until core policy decodes them.
        part.contains('%') || SENSITIVE.contains(&lower.as_str())
    });
    path_sensitive
        || url.query_pairs().any(|(name, _)| {
            let lower = name.to_ascii_lowercase().replace(['-', '_'], "");
            SENSITIVE.contains(&lower.as_str())
                || matches!(
                    lower.as_str(),
                    "code"
                        | "accesstoken"
                        | "refreshtoken"
                        | "idtoken"
                        | "sessionid"
                        | "signature"
                        | "sig"
                        | "secret"
                )
        })
}

pub(super) fn origin(value: &str) -> Option<String> {
    let url = tauri::Url::parse(value).ok()?;
    let local = url.host_str() == Some("127.0.0.1");
    (url.username().is_empty()
        && url.password().is_none()
        && (url.scheme() == "https" || (url.scheme() == "http" && local)))
        .then(|| url.origin().ascii_serialization())
}

pub(super) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> HostConfig {
        HostConfig {
            label: "browser-research-fixture".into(),
            start_url: "https://site.example/app".into(),
            profile_dir: std::env::temp_dir().join("synthetic-research-profile"),
            navigation_origins: vec!["https://site.example".into()],
            resource_origins: vec!["https://cdn.example".into()],
            api_origins: vec!["https://api.example".into()],
            identity_origins: vec!["https://login.example".into()],
            max_body_bytes: 1024,
            expires_at_ms: now_ms() + 60000,
        }
    }

    #[test]
    fn origins_separate_navigation_resources_api_and_identity() {
        let scope = config();
        assert!(scope.validate().is_ok());
        assert!(scope.allows_navigation("https://login.example/authorize"));
        assert!(!scope.allows_document("https://login.example/authorize"));
        assert!(scope.allows_resource("https://cdn.example/chunk.js"));
        assert!(!scope.allows_api("https://cdn.example/list"));
        assert!(!scope.allows_navigation("https://cdn.example/"));
        assert!(scope.allows_api("https://api.example/strategy/list"));
        assert!(!scope.allows_api("https://api.example.evil.invalid/strategy/list"));
    }

    #[test]
    fn credential_paths_queries_and_ambiguous_encoding_never_enter_capture() {
        let scope = config();
        for url in [
            "https://api.example/login",
            "https://api.example/auth/token",
            "https://api.example/%74oken",
            "https://api.example/data?access_token=synthetic",
            "https://api.example/data?signature=synthetic",
            "https://api.example/sign-in",
        ] {
            assert!(!scope.allows_api(url));
        }
        assert!(!scope.allows_document("https://site.example/login"));
        assert!(scope.allows_resource("https://cdn.example/login-helper.js"));
    }

    #[test]
    fn configuration_rejects_noncanonical_origins_and_unbounded_bodies() {
        let mut scope = config();
        scope.api_origins = vec!["https://api.example/path".into()];
        assert!(scope.validate().is_err());
        let mut scope = config();
        scope.max_body_bytes = 2 * 1024 * 1024 + 1;
        assert!(scope.validate().is_err());
        assert_eq!(
            origin("http://127.0.0.1:8123/test"),
            Some("http://127.0.0.1:8123".into())
        );
        assert!(origin("http://localhost:8123/test").is_none());
        assert!(origin("https://user:synthetic@site.example/").is_none());
    }

    #[test]
    fn pause_invalidates_late_callbacks_and_expiry_never_reactivates() {
        let handle = HostHandle {
            label: "browser-research-fixture".into(),
            control: Arc::new(Control {
                active: AtomicBool::new(true),
                generation: AtomicU64::new(1),
                closed: AtomicBool::new(false),
                expires_at_ms: now_ms() + 60000,
                sink: Arc::new(|_| {}),
            }),
        };
        assert!(handle.accepts(1));
        handle.pause();
        assert!(!handle.accepts(1));
        handle.control.active.store(true, Ordering::SeqCst);
        assert!(!handle.accepts(1));
        assert!(handle.accepts(2));
        handle.control.closed.store(true, Ordering::SeqCst);
        assert!(!handle.active());
    }
}
