use super::auth::PreparedAuth;
use super::{
    CaptureDiagnostic, CaptureEvidenceInput, CaptureInteractionStep, CaptureScope, CaptureViewport,
    CaptureWait, PwaCaptureInput,
};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

const MAX_CONFIG_BYTES: u64 = 32 * 1024;
const MAX_PIXELS: f64 = 40_000_000.0;

#[derive(Debug)]
pub(super) struct PreparedCapture {
    pub(super) project_root: PathBuf,
    pub(super) url: Url,
    pub(super) allowed_origins: BTreeSet<String>,
    pub(super) viewport: CaptureViewport,
    pub(super) wait_for: CaptureWait,
    pub(super) capture: CaptureScope,
    pub(super) steps: Vec<CaptureInteractionStep>,
    pub(super) interaction_timeout_ms: u64,
    pub(super) auth: PreparedAuth,
    pub(super) fixture: super::fixture::PreparedFixture,
    pub(super) evidence: CaptureEvidence,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SanitizedRoute {
    pub(super) sanitized_url: String,
    pub(super) origin: String,
    pub(super) path: String,
    pub(super) query_keys: Vec<String>,
    pub(super) fragment_present: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CaptureEvidence {
    pub(super) source_revision: String,
    pub(super) route_revision: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectRuntimeConfig {
    #[serde(default)]
    allowed_origins: Vec<String>,
    #[serde(default)]
    default_auth_profile: Option<String>,
    #[serde(default)]
    default_fixture_profile: Option<String>,
    #[serde(default)]
    authenticated_ready_selector: Option<String>,
}

pub(super) fn prepare(
    project_root: &str,
    input: PwaCaptureInput,
) -> Result<PreparedCapture, CaptureDiagnostic> {
    let project_root = PathBuf::from(project_root)
        .canonicalize()
        .map_err(|_| invalid("PROJECT_ROOT_INVALID", "本机项目目录不存在或无法规范化"))?;
    if !project_root.is_dir() {
        return Err(invalid("PROJECT_ROOT_INVALID", "本机项目路径不是目录"));
    }
    validate_viewport(&input.viewport)?;
    validate_wait(&input.wait_for)?;
    validate_capture(&input.capture)?;
    let interaction_timeout_ms = validate_steps(&input.steps)?;
    let config = read_config(&project_root)?;
    let (url, allowed_origins) = validate_url(&input.url, &config.allowed_origins)?;
    let auth = super::auth::prepare_auth(
        &project_root,
        input.auth_profile.or(config.default_auth_profile),
        config.authenticated_ready_selector,
        &url,
    )?;
    let fixture = super::fixture::prepare_fixture(
        &project_root,
        input.fixture_profile.or(config.default_fixture_profile),
    )?;
    if fixture
        .local_storage
        .keys()
        .any(|key| auth.local_storage.contains_key(key))
    {
        return Err(invalid(
            "FIXTURE_PROFILE_CONFLICT",
            "fixtureProfile 与 authProfile 使用了相同 localStorage key",
        ));
    }
    let evidence = validate_evidence(input.evidence)?;
    Ok(PreparedCapture {
        project_root,
        url,
        allowed_origins,
        viewport: input.viewport,
        wait_for: input.wait_for,
        capture: input.capture,
        steps: input.steps,
        interaction_timeout_ms,
        auth,
        fixture,
        evidence,
    })
}

fn validate_steps(steps: &[CaptureInteractionStep]) -> Result<u64, CaptureDiagnostic> {
    if steps.len() > 32 {
        return Err(invalid(
            "INTERACTION_STEPS_INVALID",
            "PWA 交互步骤最多允许 32 项",
        ));
    }
    let mut timeout_ms = 0_u64;
    for step in steps {
        match step {
            CaptureInteractionStep::Click { selector } => validate_selector(Some(selector))?,
            CaptureInteractionStep::WaitFor {
                selector,
                state,
                timeout_ms: step_timeout,
            } => {
                validate_selector(Some(selector))?;
                if !matches!(state.as_str(), "attached" | "visible" | "hidden")
                    || !(100..=30_000).contains(step_timeout)
                {
                    return Err(invalid(
                        "INTERACTION_STEPS_INVALID",
                        "waitFor state 或 timeoutMs 超出允许范围",
                    ));
                }
                timeout_ms = timeout_ms.saturating_add(*step_timeout);
            }
            CaptureInteractionStep::AssertText { selector, text } => {
                validate_selector(Some(selector))?;
                if text.is_empty() || text.chars().count() > 500 || text.contains(['\r', '\0']) {
                    return Err(invalid(
                        "INTERACTION_STEPS_INVALID",
                        "assertText 文本为空、过长或包含控制字符",
                    ));
                }
            }
        }
    }
    if timeout_ms > 120_000 {
        return Err(invalid(
            "INTERACTION_STEPS_INVALID",
            "PWA 交互等待总时长不能超过 120 秒",
        ));
    }
    Ok(timeout_ms)
}

fn validate_viewport(viewport: &CaptureViewport) -> Result<(), CaptureDiagnostic> {
    if !(240..=4096).contains(&viewport.width)
        || !(240..=4096).contains(&viewport.height)
        || !viewport.device_scale_factor.is_finite()
        || !(0.5..=4.0).contains(&viewport.device_scale_factor)
    {
        return Err(invalid(
            "VIEWPORT_OUT_OF_RANGE",
            "viewport 必须在 240..4096，deviceScaleFactor 必须在 0.5..4",
        ));
    }
    let pixels = f64::from(viewport.width)
        * f64::from(viewport.height)
        * viewport.device_scale_factor.powi(2);
    if pixels > MAX_PIXELS {
        return Err(invalid(
            "VIEWPORT_PIXEL_LIMIT",
            "viewport 的实际像素总量超过 4000 万上限",
        ));
    }
    Ok(())
}

fn validate_wait(wait: &CaptureWait) -> Result<(), CaptureDiagnostic> {
    if !matches!(
        wait.condition.as_str(),
        "domcontentloaded" | "load" | "networkidle"
    ) || !(500..=120_000).contains(&wait.timeout_ms)
        || wait.settle_ms > 5_000
    {
        return Err(invalid(
            "WAIT_POLICY_INVALID",
            "等待条件、超时或稳定窗口超过允许范围",
        ));
    }
    validate_selector(wait.selector.as_deref())
}

fn validate_capture(capture: &CaptureScope) -> Result<(), CaptureDiagnostic> {
    if capture.full_page && capture.selector.is_some() {
        return Err(invalid(
            "CAPTURE_SCOPE_CONFLICT",
            "fullPage 与 capture.selector 不能同时使用",
        ));
    }
    validate_selector(capture.selector.as_deref())
}

pub(super) fn validate_selector(value: Option<&str>) -> Result<(), CaptureDiagnostic> {
    if value.is_some_and(|value| {
        let value = value.trim();
        value.is_empty() || value.len() > 1_000 || value.contains('\0')
    }) {
        return Err(invalid(
            "SELECTOR_INVALID",
            "selector 必须是 1 到 1000 字节且不含 NUL",
        ));
    }
    Ok(())
}

fn read_config(root: &Path) -> Result<ProjectRuntimeConfig, CaptureDiagnostic> {
    let path = root.join(".elon").join("ui-pwa-runtime.json");
    if !path.exists() {
        return Ok(ProjectRuntimeConfig::default());
    }
    let canonical_path = path.canonicalize().map_err(|_| {
        invalid(
            "RUNTIME_CONFIG_INVALID",
            "无法规范化 PWA Runtime 项目配置路径",
        )
    })?;
    if !canonical_path.starts_with(root)
        || fs::symlink_metadata(&path)
            .map(|value| value.file_type().is_symlink())
            .unwrap_or(true)
    {
        return Err(invalid(
            "RUNTIME_CONFIG_PATH_REJECTED",
            "PWA Runtime 项目配置必须是项目目录内的非链接文件",
        ));
    }
    let metadata = fs::metadata(&path)
        .map_err(|_| invalid("RUNTIME_CONFIG_INVALID", "无法读取 PWA Runtime 项目配置"))?;
    if metadata.len() > MAX_CONFIG_BYTES {
        return Err(invalid(
            "RUNTIME_CONFIG_INVALID",
            "PWA Runtime 项目配置超过 32KiB",
        ));
    }
    let config: ProjectRuntimeConfig = serde_json::from_slice(
        &fs::read(canonical_path)
            .map_err(|_| invalid("RUNTIME_CONFIG_INVALID", "无法读取 PWA Runtime 项目配置"))?,
    )
    .map_err(|_| invalid("RUNTIME_CONFIG_INVALID", "PWA Runtime 项目配置 JSON 无效"))?;
    if config.allowed_origins.len() > 16 {
        return Err(invalid(
            "RUNTIME_CONFIG_INVALID",
            "allowedOrigins 最多允许 16 项",
        ));
    }
    validate_selector(config.authenticated_ready_selector.as_deref())?;
    if config
        .default_auth_profile
        .as_deref()
        .is_some_and(|value| !valid_profile(value))
    {
        return Err(invalid(
            "RUNTIME_CONFIG_INVALID",
            "defaultAuthProfile 名称无效",
        ));
    }
    if config
        .default_fixture_profile
        .as_deref()
        .is_some_and(|value| !valid_profile(value))
    {
        return Err(invalid(
            "RUNTIME_CONFIG_INVALID",
            "defaultFixtureProfile 名称无效",
        ));
    }
    Ok(config)
}

fn validate_url(
    raw: &str,
    configured_origins: &[String],
) -> Result<(Url, BTreeSet<String>), CaptureDiagnostic> {
    if raw.len() > 4_096 || raw.contains(['\n', '\r', '\0']) {
        return Err(invalid("URL_INVALID", "PWA URL 为空、过长或包含控制字符"));
    }
    let url = Url::parse(raw).map_err(|_| invalid("URL_INVALID", "PWA URL 不是绝对 URL"))?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(invalid(
            "URL_SCHEME_REJECTED",
            "只允许无 userinfo 的绝对 http(s) PWA URL",
        ));
    }
    if url
        .query_pairs()
        .any(|(key, _)| sensitive_query_key(key.as_ref()))
    {
        return Err(invalid(
            "URL_SECRET_QUERY_REJECTED",
            "URL 包含疑似秘密 query；请改用本机 authProfile 会话文件",
        ));
    }
    let target_origin = origin(&url)?;
    let mut allowed = BTreeSet::new();
    for configured in configured_origins {
        let parsed = Url::parse(configured)
            .map_err(|_| invalid("RUNTIME_CONFIG_INVALID", "allowedOrigins 包含无效 URL"))?;
        if !matches!(parsed.scheme(), "http" | "https")
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.path() != "/"
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err(invalid(
                "RUNTIME_CONFIG_INVALID",
                "allowedOrigins 必须是无路径、query、fragment 和 userinfo 的 http(s) origin",
            ));
        }
        allowed.insert(origin(&parsed)?);
    }
    if is_loopback(&url) {
        allowed.insert(target_origin.clone());
    } else if !allowed.contains(&target_origin) {
        return Err(CaptureDiagnostic::new(
            "URL_ORIGIN_NOT_ALLOWED",
            "默认只允许 localhost/loopback；当前 origin 未在项目白名单中",
            false,
            "在项目 .elon/ui-pwa-runtime.json 的 allowedOrigins 中显式登记可信 origin，或改用本机预览 URL",
        ));
    }
    Ok((url, allowed))
}

pub(super) fn sanitize_url(url: &Url) -> Result<SanitizedRoute, CaptureDiagnostic> {
    let origin = origin(url)?;
    let mut query_keys = url
        .query_pairs()
        .map(|(key, _)| key.into_owned())
        .collect::<Vec<_>>();
    query_keys.sort();
    query_keys.dedup();
    let query = query_keys
        .iter()
        .map(|key| format!("{key}=[REDACTED]"))
        .collect::<Vec<_>>()
        .join("&");
    let mut sanitized_url = format!("{origin}{}", url.path());
    if !query.is_empty() {
        sanitized_url.push('?');
        sanitized_url.push_str(&query);
    }
    if url.fragment().is_some() {
        sanitized_url.push_str("#[REDACTED]");
    }
    Ok(SanitizedRoute {
        sanitized_url,
        origin,
        path: url.path().to_string(),
        query_keys,
        fragment_present: url.fragment().is_some(),
    })
}

pub(super) fn origin(url: &Url) -> Result<String, CaptureDiagnostic> {
    let value = url.origin().ascii_serialization();
    if value == "null" {
        Err(invalid("URL_INVALID", "PWA URL 没有可验证 origin"))
    } else {
        Ok(value)
    }
}

fn is_loopback(url: &Url) -> bool {
    matches!(url.host_str(), Some("localhost"))
        || url
            .host_str()
            .and_then(|host| host.parse::<std::net::IpAddr>().ok())
            .is_some_and(|ip| ip.is_loopback())
}

fn sensitive_query_key(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "token",
        "secret",
        "password",
        "passwd",
        "authorization",
        "credential",
        "session",
        "access_key",
        "api_key",
        "apikey",
        "signature",
        "jwt",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

pub(super) fn valid_profile(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
}

fn validate_evidence(input: CaptureEvidenceInput) -> Result<CaptureEvidence, CaptureDiagnostic> {
    if !safe_revision(&input.route_revision) {
        return Err(invalid(
            "REVISION_INVALID",
            "routeRevision 必须是 1..160 位安全 revision 标识",
        ));
    }
    let source_revision = if !input.source_revisions.is_empty() {
        if input.source_revisions.len() > 64 {
            return Err(invalid(
                "REVISION_INVALID",
                "sourceRevisions 最多包含 64 个文件",
            ));
        }
        for (file, revision) in &input.source_revisions {
            if safe_relative(file).is_none()
                || revision.len() != 64
                || !revision.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(invalid(
                    "REVISION_INVALID",
                    "sourceRevisions 包含越界路径或非 SHA-256 revision",
                ));
            }
        }
        let bytes = serde_json::to_vec(&input.source_revisions)
            .map_err(|_| invalid("REVISION_INVALID", "无法规范化 sourceRevisions"))?;
        format!(
            "pwa-source-set-sha256:{}",
            hex::encode(Sha256::digest(bytes))
        )
    } else {
        input
            .source_revision
            .filter(|value| safe_revision(value))
            .ok_or_else(|| {
                invalid(
                    "REVISION_REQUIRED",
                    "必须提供安全 sourceRevision 或 sourceRevisions",
                )
            })?
    };
    Ok(CaptureEvidence {
        source_revision,
        route_revision: input.route_revision,
    })
}

fn safe_revision(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | ':' | '-'))
}

fn safe_relative(value: &str) -> Option<PathBuf> {
    let path = Path::new(value);
    if path.is_absolute() || value.is_empty() || value.len() > 500 || value.contains('\0') {
        return None;
    }
    path.components()
        .all(|component| matches!(component, Component::Normal(_)))
        .then(|| path.to_path_buf())
}

pub(super) fn invalid(code: &'static str, message: impl Into<String>) -> CaptureDiagnostic {
    CaptureDiagnostic::new(code, message, false, "修正参数或项目本机配置后重试")
}
