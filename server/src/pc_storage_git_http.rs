// server/src/pc_storage_git_http.rs

use axum::{
    body::Body,
    http::{HeaderName, HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Stdio,
};

use crate::pc_storage_repo::{self, StorageSettings};

const DEFAULT_BODY_LIMIT: usize = 128 * 1024 * 1024;

pub async fn handle_git_http(settings: StorageSettings, req: Request<Body>) -> Response {
    if !settings.enabled {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "storage service is disabled",
        )
            .into_response();
    }
    let method = req.method().to_string();
    let uri = req.uri().clone();
    let headers = req.headers().clone();
    let body = match axum::body::to_bytes(req.into_body(), request_body_limit()).await {
        Ok(body) => body.to_vec(),
        Err(err) => return (StatusCode::PAYLOAD_TOO_LARGE, err.to_string()).into_response(),
    };
    let Some((token, path_info)) = split_storage_git_path(uri.path()) else {
        return (StatusCode::NOT_FOUND, "invalid storage git path").into_response();
    };
    if !valid_token_shape(token) {
        return (StatusCode::UNAUTHORIZED, "invalid storage git token").into_response();
    }

    let git_root = pc_storage_repo::git_project_root(&settings);
    let repo = match repo_for_path_info(&git_root, &path_info) {
        Ok(repo) => repo,
        Err(status) => return (status, "invalid storage git repository path").into_response(),
    };
    if !repo.join("HEAD").exists() {
        return (StatusCode::NOT_FOUND, "storage git repository not found").into_response();
    }
    if !pc_storage_repo::validate_repo_access_token(&repo, token) {
        return (StatusCode::UNAUTHORIZED, "storage git token rejected").into_response();
    }

    let query = uri.query().unwrap_or("").to_string();
    let content_type = header_value(&headers, axum::http::header::CONTENT_TYPE);
    let git_protocol = header_value(&headers, HeaderName::from_static("git-protocol"));
    let path_info = path_info.to_string();
    let backend = tokio::task::spawn_blocking(move || {
        run_git_http_backend(
            git_root,
            method,
            path_info,
            query,
            content_type,
            git_protocol,
            body,
        )
    })
    .await;

    match backend {
        Ok(Ok(response)) => response,
        Ok(Err(message)) => (StatusCode::BAD_GATEWAY, message).into_response(),
        Err(err) => (StatusCode::BAD_GATEWAY, err.to_string()).into_response(),
    }
}

fn run_git_http_backend(
    git_root: PathBuf,
    method: String,
    path_info: String,
    query: String,
    content_type: Option<String>,
    git_protocol: Option<String>,
    body: Vec<u8>,
) -> Result<Response, String> {
    let mut command = crate::git_command_error::git_command();
    command
        .arg("http-backend")
        .env("GIT_PROJECT_ROOT", git_root)
        .env("GIT_HTTP_EXPORT_ALL", "1")
        .env("REQUEST_METHOD", method)
        .env("PATH_INFO", path_info)
        .env("QUERY_STRING", query)
        .env("CONTENT_LENGTH", body.len().to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(content_type) = content_type {
        command.env("CONTENT_TYPE", content_type);
    }
    if let Some(git_protocol) = git_protocol {
        command.env("GIT_PROTOCOL", git_protocol);
    }

    let mut child = command
        .spawn()
        .map_err(|err| format!("failed to start git http-backend: {err}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(&body)
            .map_err(|err| format!("failed to write git request body: {err}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|err| format!("failed to run git http-backend: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "git http-backend failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse_cgi_response(&output.stdout)
}

fn parse_cgi_response(raw: &[u8]) -> Result<Response, String> {
    let Some((head, body)) = split_cgi_response(raw) else {
        return Err("git http-backend returned malformed CGI response".into());
    };
    let head = String::from_utf8_lossy(head);
    let mut status = StatusCode::OK;
    let mut builder = Response::builder();
    for line in head.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("Status") {
            if let Some(code) = value
                .split_whitespace()
                .next()
                .and_then(|code| code.parse::<u16>().ok())
            {
                status = StatusCode::from_u16(code).unwrap_or(StatusCode::OK);
            }
            continue;
        }
        if let (Ok(name), Ok(value)) = (
            name.parse::<HeaderName>(),
            value.trim().parse::<HeaderValue>(),
        ) {
            builder = builder.header(name, value);
        }
    }
    builder
        .status(status)
        .body(Body::from(body.to_vec()))
        .map_err(|err| err.to_string())
}

fn split_cgi_response(raw: &[u8]) -> Option<(&[u8], &[u8])> {
    if let Some(index) = raw.windows(4).position(|window| window == b"\r\n\r\n") {
        return Some((&raw[..index], &raw[index + 4..]));
    }
    raw.windows(2)
        .position(|window| window == b"\n\n")
        .map(|index| (&raw[..index], &raw[index + 2..]))
}

fn split_storage_git_path(path: &str) -> Option<(&str, String)> {
    let rest = path.strip_prefix("/storage/git/")?;
    let (token, path_info) = rest.split_once('/')?;
    Some((token, format!("/{path_info}")))
}

fn repo_for_path_info(git_root: &Path, path_info: &str) -> Result<PathBuf, StatusCode> {
    if !path_info.starts_with("/projects/") || path_info.contains('\\') {
        return Err(StatusCode::NOT_FOUND);
    }
    if path_info.split('/').any(|part| part == "..") {
        return Err(StatusCode::NOT_FOUND);
    }
    let Some(end) = path_info.find(".git").map(|index| index + ".git".len()) else {
        return Err(StatusCode::NOT_FOUND);
    };
    let separator = std::path::MAIN_SEPARATOR.to_string();
    let repo_rel = path_info[1..end].replace('/', &separator);
    let repo = git_root.join(repo_rel);
    ensure_repo_within_root(git_root, &repo)?;
    Ok(repo)
}

fn ensure_repo_within_root(git_root: &Path, repo: &Path) -> Result<(), StatusCode> {
    let root = std::fs::canonicalize(git_root).map_err(|_| StatusCode::NOT_FOUND)?;
    let repo = std::fs::canonicalize(repo).map_err(|_| StatusCode::NOT_FOUND)?;
    if repo.starts_with(root) {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

fn header_value(headers: &axum::http::HeaderMap, name: HeaderName) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

fn valid_token_shape(token: &str) -> bool {
    token.len() >= 32
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
}

fn request_body_limit() -> usize {
    std::env::var("ELON_STORAGE_GIT_LOCAL_BODY_LIMIT_MB")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .and_then(|mb| mb.checked_mul(1024 * 1024))
        .unwrap_or(DEFAULT_BODY_LIMIT)
}

#[cfg(test)]
mod tests {
    use super::{repo_for_path_info, split_storage_git_path};
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn split_storage_git_path_keeps_project_prefix() {
        let (token, path_info) = split_storage_git_path(
            "/storage/git/abcdefghijklmnopqrstuvwxyz0123456789/projects/user/project.git/info/refs",
        )
        .expect("path should split");
        assert_eq!(token, "abcdefghijklmnopqrstuvwxyz0123456789");
        assert_eq!(path_info, "/projects/user/project.git/info/refs");
    }

    #[test]
    fn repo_for_path_info_rejects_traversal() {
        let root =
            std::env::temp_dir().join(format!("elon_storage_git_http_{}", Uuid::new_v4().simple()));
        let repo = root.join("projects").join("user").join("project.git");
        fs::create_dir_all(&repo).expect("repo dir should create");
        fs::write(repo.join("HEAD"), "ref: refs/heads/main\n").expect("head should write");

        let resolved = repo_for_path_info(&root, "/projects/user/project.git/info/refs")
            .expect("repo path should resolve");
        assert_eq!(resolved, repo);
        assert!(repo_for_path_info(&root, "/projects/../secret.git/info/refs").is_err());
        let _ = fs::remove_dir_all(root);
    }
}
