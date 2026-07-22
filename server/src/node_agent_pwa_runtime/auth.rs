use super::{
    security::{invalid, valid_profile, validate_selector},
    CaptureDiagnostic,
};
use reqwest::Url;
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::Path};

const MAX_SESSION_BYTES: u64 = 64 * 1024;

#[derive(Debug)]
pub(super) struct PreparedAuth {
    pub(super) mode: &'static str,
    pub(super) profile: Option<String>,
    pub(super) cookies: Vec<PreparedCookie>,
    pub(super) headers: BTreeMap<String, String>,
    pub(super) local_storage: BTreeMap<String, String>,
    pub(super) ready_selector: Option<String>,
}

#[derive(Debug)]
pub(super) struct PreparedCookie {
    pub(super) name: String,
    pub(super) value: String,
    pub(super) path: String,
    pub(super) http_only: bool,
    pub(super) secure: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SessionFile {
    version: u8,
    #[serde(default)]
    cookies: Vec<SessionCookie>,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    local_storage: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SessionCookie {
    name: String,
    value: String,
    #[serde(default = "default_cookie_path")]
    path: String,
    #[serde(default)]
    http_only: bool,
    #[serde(default)]
    secure: bool,
}

fn default_cookie_path() -> String {
    "/".to_string()
}

pub(super) fn prepare_auth(
    root: &Path,
    profile: Option<String>,
    ready_selector: Option<String>,
    target: &Url,
) -> Result<PreparedAuth, CaptureDiagnostic> {
    validate_selector(ready_selector.as_deref())?;
    let Some(profile) = profile else {
        return Ok(PreparedAuth {
            mode: "none",
            profile: None,
            cookies: Vec::new(),
            headers: BTreeMap::new(),
            local_storage: BTreeMap::new(),
            ready_selector,
        });
    };
    if !valid_profile(&profile) {
        return Err(invalid(
            "AUTH_PROFILE_INVALID",
            "authProfile 只能包含 1..64 位字母、数字、下划线或连字符",
        ));
    }
    let path = root
        .join(".elon")
        .join("ui-tuner")
        .join("pwa-sessions")
        .join(format!("{profile}.json"));
    let metadata = fs::metadata(&path).map_err(|_| {
        CaptureDiagnostic::new(
            "AUTH_PROFILE_NOT_PREPARED",
            "指定的 PWA authProfile 尚未在本机准备",
            false,
            format!(
                "创建 .elon/ui-tuner/pwa-sessions/{profile}.json；只保存于本机并确保该目录不进入 Git"
            ),
        )
    })?;
    let canonical_path = path.canonicalize().map_err(|_| {
        invalid(
            "AUTH_PROFILE_PATH_REJECTED",
            "无法规范化 PWA authProfile 文件路径",
        )
    })?;
    if !canonical_path.starts_with(root)
        || fs::symlink_metadata(&path)
            .map(|value| value.file_type().is_symlink())
            .unwrap_or(true)
    {
        return Err(invalid(
            "AUTH_PROFILE_PATH_REJECTED",
            "PWA authProfile 必须是项目目录内的非链接文件",
        ));
    }
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_SESSION_BYTES {
        return Err(invalid(
            "AUTH_PROFILE_INVALID",
            "PWA authProfile 文件必须是 1..64KiB 的普通 JSON 文件",
        ));
    }
    let session: SessionFile = serde_json::from_slice(
        &fs::read(&canonical_path)
            .map_err(|_| invalid("AUTH_PROFILE_INVALID", "无法读取 authProfile"))?,
    )
    .map_err(|_| invalid("AUTH_PROFILE_INVALID", "authProfile JSON 无效"))?;
    if session.version != 1
        || session.cookies.len() > 64
        || session.headers.len() > 16
        || session.local_storage.len() > 16
    {
        return Err(invalid(
            "AUTH_PROFILE_INVALID",
            "authProfile 必须是 version=1，最多 64 个 Cookie、16 个 header 和 16 个 localStorage 项",
        ));
    }
    let mut cookies = Vec::new();
    for cookie in session.cookies {
        if !valid_cookie_name(&cookie.name)
            || cookie.value.is_empty()
            || cookie.value.len() > 4_096
            || !cookie.path.starts_with('/')
            || cookie.path.len() > 512
            || cookie.value.contains(['\r', '\n', '\0'])
        {
            return Err(invalid(
                "AUTH_PROFILE_INVALID",
                "authProfile 包含无效 Cookie 字段",
            ));
        }
        cookies.push(PreparedCookie {
            name: cookie.name,
            value: cookie.value,
            path: cookie.path,
            http_only: cookie.http_only,
            secure: cookie.secure || target.scheme() == "https",
        });
    }
    for (name, value) in &session.headers {
        let lower = name.to_ascii_lowercase();
        if !(lower == "authorization" || lower.starts_with("x-"))
            || !name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
            || value.is_empty()
            || value.len() > 8_192
            || value.contains(['\r', '\n', '\0'])
        {
            return Err(invalid(
                "AUTH_PROFILE_INVALID",
                "authProfile 只允许 Authorization 或 X-* header，且值不得包含控制字符",
            ));
        }
    }
    for (name, value) in &session.local_storage {
        if name.is_empty()
            || name.len() > 256
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
            || value.is_empty()
            || value.len() > 8_192
            || value.contains(['\r', '\n', '\0'])
        {
            return Err(invalid(
                "AUTH_PROFILE_INVALID",
                "authProfile localStorage 键值无效或包含控制字符",
            ));
        }
    }
    Ok(PreparedAuth {
        mode: "prepared_profile",
        profile: Some(profile),
        cookies,
        headers: session.headers,
        local_storage: session.local_storage,
        ready_selector,
    })
}

fn valid_cookie_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte > 0x20 && byte < 0x7f && !b"()<>@,;:\\\"/[]?={}".contains(&byte))
}
