use crate::{node_agent_atomic_file, NodeRuntime};
use axum::{
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};

const AUTO_PROFILE_PREFIX: &str = "pc_ui_tuner_";
const AUTO_PROFILE_ID_LEN: usize = 32;
const PROFILE_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_PROJECT_ROOT_BYTES: usize = 4_096;
const MAX_TOKEN_BYTES: usize = 8_192 - "Bearer ".len();

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PrepareRequest {
    project_root: String,
    token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CleanupRequest {
    project_root: String,
    profile: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrepareResponse {
    profile: String,
    expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CleanupResponse {
    cleaned: bool,
}

#[derive(Debug)]
struct ProfileError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
}

impl ProfileError {
    fn invalid(code: &'static str, message: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message,
        }
    }

    fn internal(code: &'static str, message: &'static str) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code,
            message,
        }
    }

    fn response(self) -> Response {
        (
            self.status,
            Json(json!({ "code": self.code, "error": self.message })),
        )
            .into_response()
    }
}

pub(crate) fn routes() -> Router<Arc<NodeRuntime>> {
    Router::new()
        .route(
            "/api/source-preview/pwa-auth-profile/prepare",
            post(prepare_handler),
        )
        .route(
            "/api/source-preview/pwa-auth-profile/cleanup",
            post(cleanup_handler),
        )
}

async fn prepare_handler(payload: Result<Json<PrepareRequest>, JsonRejection>) -> Response {
    let Json(request) = match payload {
        Ok(request) => request,
        Err(_) => {
            return ProfileError::invalid(
                "PWA_AUTH_PROFILE_REQUEST_INVALID",
                "临时 PWA 登录态准备请求无效",
            )
            .response()
        }
    };
    match prepare_profile(request) {
        Ok(response) => Json(response).into_response(),
        Err(error) => error.response(),
    }
}

async fn cleanup_handler(payload: Result<Json<CleanupRequest>, JsonRejection>) -> Response {
    let Json(request) = match payload {
        Ok(request) => request,
        Err(_) => {
            return ProfileError::invalid(
                "PWA_AUTH_PROFILE_REQUEST_INVALID",
                "临时 PWA 登录态清理请求无效",
            )
            .response()
        }
    };
    match cleanup_profile(request) {
        Ok(cleaned) => Json(CleanupResponse { cleaned }).into_response(),
        Err(error) => error.response(),
    }
}

fn prepare_profile(request: PrepareRequest) -> Result<PrepareResponse, ProfileError> {
    validate_token(&request.token)?;
    let project_root = canonical_project_root(&request.project_root)?;
    let session_dir = scoped_session_dir(&project_root, true)?.ok_or_else(|| {
        ProfileError::internal(
            "PWA_AUTH_PROFILE_DIRECTORY_FAILED",
            "无法创建临时 PWA 登录态目录",
        )
    })?;
    evict_expired_profiles_at(&session_dir, SystemTime::now(), PROFILE_TTL, |path| {
        fs::remove_file(path)
    })?;

    let profile = format!("{AUTO_PROFILE_PREFIX}{}", uuid::Uuid::new_v4().simple());
    let path = session_dir.join(format!("{profile}.json"));
    let bytes = serde_json::to_vec(&json!({
        "version": 1,
        "cookies": [],
        "headers": { "Authorization": format!("Bearer {}", request.token) },
    }))
    .map_err(|_| {
        ProfileError::internal("PWA_AUTH_PROFILE_WRITE_FAILED", "无法准备临时 PWA 登录态")
    })?;
    node_agent_atomic_file::write_new(&path, &bytes).map_err(|_| {
        ProfileError::internal("PWA_AUTH_PROFILE_WRITE_FAILED", "无法准备临时 PWA 登录态")
    })?;
    if verify_profile_path(&session_dir, &path).is_err() {
        let _ = fs::remove_file(&path);
        return Err(ProfileError::internal(
            "PWA_AUTH_PROFILE_PATH_REJECTED",
            "临时 PWA 登录态路径未通过安全检查",
        ));
    }
    let expires_at = DateTime::<Utc>::from(SystemTime::now() + PROFILE_TTL);
    Ok(PrepareResponse {
        profile,
        expires_at,
    })
}

fn cleanup_profile(request: CleanupRequest) -> Result<bool, ProfileError> {
    cleanup_profile_with(request, |path| fs::remove_file(path))
}

fn cleanup_profile_with(
    request: CleanupRequest,
    remove: impl Fn(&Path) -> std::io::Result<()>,
) -> Result<bool, ProfileError> {
    if !valid_auto_profile(&request.profile) {
        return Err(ProfileError::invalid(
            "PWA_AUTH_PROFILE_CLEANUP_REJECTED",
            "只允许清理服务端生成的临时 PWA 登录态",
        ));
    }
    let project_root = canonical_project_root(&request.project_root)?;
    let Some(session_dir) = scoped_session_dir(&project_root, false)? else {
        return Ok(false);
    };
    let path = session_dir.join(format!("{}.json", request.profile));
    match fs::symlink_metadata(&path) {
        Ok(_) => verify_profile_path(&session_dir, &path)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(_) => {
            return Err(ProfileError::internal(
                "PWA_AUTH_PROFILE_CLEANUP_FAILED",
                "无法检查临时 PWA 登录态",
            ))
        }
    }
    match remove(&path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(ProfileError::internal(
            "PWA_AUTH_PROFILE_CLEANUP_FAILED",
            "无法清理临时 PWA 登录态",
        )),
    }
}

fn validate_token(token: &str) -> Result<(), ProfileError> {
    if token.is_empty() || token.len() > MAX_TOKEN_BYTES || token.contains(['\r', '\n', '\0']) {
        return Err(ProfileError::invalid(
            "PWA_AUTH_TOKEN_INVALID",
            "当前登录态缺失、过长或包含非法控制字符",
        ));
    }
    Ok(())
}

fn canonical_project_root(value: &str) -> Result<PathBuf, ProfileError> {
    if value.is_empty()
        || value.len() > MAX_PROJECT_ROOT_BYTES
        || value.contains(['\r', '\n', '\0'])
    {
        return Err(ProfileError::invalid(
            "PROJECT_ROOT_INVALID",
            "本机项目目录为空、过长或包含非法控制字符",
        ));
    }
    let root = PathBuf::from(value).canonicalize().map_err(|_| {
        ProfileError::invalid("PROJECT_ROOT_INVALID", "本机项目目录不存在或无法规范化")
    })?;
    if !root.is_dir() {
        return Err(ProfileError::invalid(
            "PROJECT_ROOT_INVALID",
            "本机项目路径不是目录",
        ));
    }
    Ok(root)
}

fn scoped_session_dir(root: &Path, create: bool) -> Result<Option<PathBuf>, ProfileError> {
    let requested = root.join(".elon").join("ui-tuner").join("pwa-sessions");
    let mut existing = requested.as_path();
    while !existing.exists() {
        existing = existing.parent().ok_or_else(path_rejected)?;
    }
    let canonical_existing = existing.canonicalize().map_err(|_| path_rejected())?;
    if canonical_existing != existing || !canonical_existing.starts_with(root) {
        return Err(path_rejected());
    }
    if !create && !requested.exists() {
        return Ok(None);
    }
    if create {
        fs::create_dir_all(&requested).map_err(|_| {
            ProfileError::internal(
                "PWA_AUTH_PROFILE_DIRECTORY_FAILED",
                "无法创建临时 PWA 登录态目录",
            )
        })?;
    }
    let canonical = requested.canonicalize().map_err(|_| path_rejected())?;
    if canonical != requested || !canonical.starts_with(root) || !canonical.is_dir() {
        return Err(path_rejected());
    }
    Ok(Some(canonical))
}

fn verify_profile_path(session_dir: &Path, path: &Path) -> Result<(), ProfileError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| path_rejected())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(path_rejected());
    }
    let canonical = path.canonicalize().map_err(|_| path_rejected())?;
    if canonical != path || canonical.parent() != Some(session_dir) {
        return Err(path_rejected());
    }
    Ok(())
}

fn path_rejected() -> ProfileError {
    ProfileError::invalid(
        "PWA_AUTH_PROFILE_PATH_REJECTED",
        "临时 PWA 登录态路径越出项目或经过链接/重解析点",
    )
}

fn valid_auto_profile(value: &str) -> bool {
    value.strip_prefix(AUTO_PROFILE_PREFIX).is_some_and(|id| {
        id.len() == AUTO_PROFILE_ID_LEN && id.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn evict_expired_profiles_at(
    session_dir: &Path,
    now: SystemTime,
    ttl: Duration,
    remove: impl Fn(&Path) -> std::io::Result<()>,
) -> Result<(), ProfileError> {
    let entries = fs::read_dir(session_dir).map_err(|_| {
        ProfileError::internal(
            "PWA_AUTH_PROFILE_CLEANUP_FAILED",
            "无法扫描临时 PWA 登录态目录",
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|_| {
            ProfileError::internal(
                "PWA_AUTH_PROFILE_CLEANUP_FAILED",
                "无法扫描临时 PWA 登录态目录",
            )
        })?;
        let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let Some(profile) = file_name.strip_suffix(".json") else {
            continue;
        };
        if !valid_auto_profile(profile) {
            continue;
        }
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|_| {
            ProfileError::internal(
                "PWA_AUTH_PROFILE_CLEANUP_FAILED",
                "无法检查过期临时 PWA 登录态",
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            continue;
        }
        let modified = metadata.modified().map_err(|_| {
            ProfileError::internal(
                "PWA_AUTH_PROFILE_CLEANUP_FAILED",
                "无法检查临时 PWA 登录态有效期",
            )
        })?;
        if now.duration_since(modified).is_ok_and(|age| age >= ttl) {
            match remove(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => {
                    return Err(ProfileError::internal(
                        "PWA_AUTH_PROFILE_CLEANUP_FAILED",
                        "无法淘汰过期临时 PWA 登录态",
                    ))
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "elon-pwa-auth-profile-{label}-{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&root).unwrap();
        root.canonicalize().unwrap()
    }

    fn auto_profile(id: char) -> String {
        format!(
            "{AUTO_PROFILE_PREFIX}{}",
            id.to_string().repeat(AUTO_PROFILE_ID_LEN)
        )
    }

    #[test]
    fn prepare_and_cleanup_keep_the_secret_out_of_responses() {
        let root = fixture_root("round-trip");
        let secret = "test-secret-that-must-not-leak";
        let prepared = prepare_profile(PrepareRequest {
            project_root: root.display().to_string(),
            token: secret.to_string(),
        })
        .unwrap();
        assert!(valid_auto_profile(&prepared.profile));
        let response = serde_json::to_string(&prepared).unwrap();
        assert!(!response.contains(secret));
        let path = root
            .join(".elon/ui-tuner/pwa-sessions")
            .join(format!("{}.json", prepared.profile));
        let stored = fs::read_to_string(&path).unwrap();
        assert!(stored.contains(&format!("Bearer {secret}")));

        let cleaned = cleanup_profile(CleanupRequest {
            project_root: root.display().to_string(),
            profile: prepared.profile,
        })
        .unwrap();
        assert!(cleaned);
        assert!(!path.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_values_are_rejected_without_echoing_the_secret() {
        let root = fixture_root("invalid");
        let secret = "secret\nshould-not-appear";
        let error = prepare_profile(PrepareRequest {
            project_root: root.display().to_string(),
            token: secret.to_string(),
        })
        .unwrap_err();
        let response = serde_json::to_string(&json!({
            "code": error.code,
            "error": error.message,
        }))
        .unwrap();
        assert!(!response.contains(secret));

        let invalid_root = format!("{}\0outside", root.display());
        assert_eq!(
            canonical_project_root(&invalid_root).unwrap_err().code,
            "PROJECT_ROOT_INVALID"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn expiry_removes_only_server_generated_profiles() {
        let root = fixture_root("expiry");
        let session_dir = scoped_session_dir(&root, true).unwrap().unwrap();
        let automatic = session_dir.join(format!("{}.json", auto_profile('a')));
        let manual = session_dir.join("manual-login.json");
        fs::write(&automatic, b"{}").unwrap();
        fs::write(&manual, b"{}").unwrap();

        evict_expired_profiles_at(&session_dir, SystemTime::now(), Duration::ZERO, |path| {
            fs::remove_file(path)
        })
        .unwrap();
        assert!(!automatic.exists());
        assert!(manual.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cleanup_rejects_manual_profiles_and_reports_failures_generically() {
        let root = fixture_root("cleanup");
        let session_dir = scoped_session_dir(&root, true).unwrap().unwrap();
        let manual = session_dir.join("manual-login.json");
        fs::write(&manual, b"{}").unwrap();
        assert_eq!(
            cleanup_profile(CleanupRequest {
                project_root: root.display().to_string(),
                profile: "manual-login".to_string(),
            })
            .unwrap_err()
            .code,
            "PWA_AUTH_PROFILE_CLEANUP_REJECTED"
        );
        assert!(manual.exists());

        let profile = auto_profile('b');
        fs::write(session_dir.join(format!("{profile}.json")), b"{}").unwrap();
        let error = cleanup_profile_with(
            CleanupRequest {
                project_root: root.display().to_string(),
                profile,
            },
            |_| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "simulated lock",
                ))
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "PWA_AUTH_PROFILE_CLEANUP_FAILED");
        assert!(!error.message.contains("simulated"));
        fs::remove_dir_all(root).unwrap();
    }
}
