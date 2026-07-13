use anyhow::{anyhow, Result};
use homecli_proto::AgentToServer;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::mpsc::UnboundedSender;

use super::{
    ai_cli_apk_build_script::pc_apk_sync_loader_command, looks_like_android_task, AiCliRequestMode,
};
use crate::{
    tools,
    types::{AppState, WsMessage},
};

#[derive(Debug)]
struct SyncedPcApk {
    path: PathBuf,
    file_name: String,
    sha256: String,
    size_bytes: i64,
    metadata: SyncedPcApkMetadata,
}

#[derive(Debug, Default)]
struct SyncedPcApkMetadata {
    package_name: Option<String>,
    version_name: Option<String>,
    version_code: Option<i64>,
    build_started_at: Option<String>,
    source_git_sha: Option<String>,
    apk_modified_at: Option<String>,
}

struct PcApkRelayOutput {
    file_name: String,
    apk_bytes: Vec<u8>,
    metadata: SyncedPcApkMetadata,
}

pub(crate) fn pc_apk_probe_since(request_mode: AiCliRequestMode, cwd: Option<&str>) -> Option<u64> {
    if request_mode.is_plan() || cwd.is_none() {
        return None;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs().saturating_sub(600))
}

pub(crate) fn pc_apk_sync_workspace<'a>(
    configured_workspace: Option<&'a str>,
    active_workspace: Option<&'a str>,
) -> Option<&'a str> {
    clean_workspace_path(active_workspace).or_else(|| clean_workspace_path(configured_workspace))
}

pub(crate) async fn sync_pc_agent_apk_after_success(
    state: &Arc<AppState>,
    agent_id: &str,
    pc_workspace: Option<&str>,
    user_message: &str,
    request_mode: AiCliRequestMode,
    attempt_apk_sync: bool,
    apk_sync_probe_since: Option<u64>,
    download_base: Option<&str>,
    artifact_workspace: Option<&Path>,
    tx: &UnboundedSender<String>,
) -> Option<String> {
    if request_mode.is_plan() {
        return None;
    }
    let (Some(pc_workspace), Some(download_base), Some(artifact_workspace)) =
        (pc_workspace, download_base, artifact_workspace)
    else {
        return None;
    };

    let explicit_apk_sync = attempt_apk_sync || looks_like_android_task(user_message);
    let fresh_after_unix_secs = apk_sync_probe_since;
    if !explicit_apk_sync && fresh_after_unix_secs.is_none() {
        return None;
    }

    if explicit_apk_sync {
        let _ = tx.send(WsMessage::progress("正在同步 PC 构建产物，准备安装入口。").to_json());
    }
    match sync_pc_agent_apk_artifact(
        state,
        agent_id,
        pc_workspace,
        artifact_workspace,
        project_id_from_download_base(download_base),
        fresh_after_unix_secs,
        explicit_apk_sync,
    )
    .await
    {
        Ok(Some(apk)) => {
            let apk_url = tools::stable_apk_url(download_base);
            register_synced_pc_release(state, download_base, &apk_url, &apk);
            Some(apk_url)
        }
        Ok(None) => {
            if explicit_apk_sync {
                let _ = tx.send(
                    WsMessage::progress("本轮 PC 工作区没有同步到新的 APK；不会复用旧安装包入口。")
                        .to_json(),
                );
            }
            None
        }
        Err(error) => {
            tracing::warn!(%agent_id, %error, "同步 PC APK 产物失败");
            if explicit_apk_sync {
                let _ = tx.send(
                    WsMessage::progress("同步 PC 构建产物失败；本轮不会生成安装按钮链接。")
                        .to_json(),
                );
            }
            None
        }
    }
}

fn clean_workspace_path(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|path| !path.is_empty())
}

fn register_synced_pc_release(
    state: &AppState,
    download_base: &str,
    apk_url: &str,
    apk: &SyncedPcApk,
) {
    let Some(project_id) = project_id_from_download_base(download_base) else {
        return;
    };
    if project_id.is_empty() {
        return;
    }
    let file_path = apk.path.to_string_lossy();
    let metadata_json = synced_pc_apk_metadata_json(&apk.metadata);
    let version_name = apk
        .metadata
        .version_name
        .as_deref()
        .unwrap_or("PC node debug build");
    if let Err(error) =
        state
            .store
            .create_project_release(crate::store::project_releases::ProjectReleaseWrite {
                id: None,
                project_id,
                task_id: None,
                uploaded_by: None,
                version_name: Some(version_name),
                package_name: apk.metadata.package_name.as_deref(),
                version_code: apk.metadata.version_code,
                channel: Some("pc_node"),
                status: Some("published"),
                apk_url,
                file_name: &apk.file_name,
                file_path: Some(file_path.as_ref()),
                sha256: Some(&apk.sha256),
                size_bytes: Some(apk.size_bytes),
                changelog: Some("Synced from PC node after AI development task"),
                build_started_at: apk.metadata.build_started_at.as_deref(),
                source_git_sha: apk.metadata.source_git_sha.as_deref(),
                source_worktree: None,
                metadata_json: metadata_json.as_deref(),
            })
    {
        tracing::warn!(
            project_id = %project_id,
            error = %error,
            "failed to register synced PC APK release"
        );
    }
}

fn project_id_from_download_base(download_base: &str) -> Option<&str> {
    let without_query = download_base.split('?').next().unwrap_or(download_base);
    let parts = without_query.split('/').collect::<Vec<_>>();
    for window in parts.windows(3) {
        if window[0] == "projects" && window[2] == "download" {
            return Some(window[1]);
        }
    }
    None
}

async fn sync_pc_agent_apk_artifact(
    state: &Arc<AppState>,
    agent_id: &str,
    pc_workspace: &str,
    artifact_workspace: &Path,
    project_id: Option<&str>,
    fresh_after_unix_secs: Option<u64>,
    build_if_missing: bool,
) -> Result<Option<SyncedPcApk>> {
    let command =
        pc_apk_sync_loader_command(&state.public_url, fresh_after_unix_secs, build_if_missing);
    let project_context = project_id.map(|project_id| homecli_proto::CliProjectContext {
        project_id: project_id.to_string(),
        conversation_id: "apk-sync".to_string(),
        runtime_permission: Some("project_write".to_string()),
    });
    let (_task_id, mut rx) = state
        .agent_manager
        .dispatch_with_project_context(
            agent_id,
            "powershell".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-Command".to_string(),
                command,
            ],
            pc_workspace.to_string(),
            vec![],
            project_context,
        )
        .await?;

    let mut output = String::new();
    let mut stderr = String::new();
    let mut exit_code = None;
    while let Some(msg) = rx.recv().await {
        match msg {
            AgentToServer::TaskStdout { data, .. } => {
                output.push_str(&pc_task_text_chunk(&data));
            }
            AgentToServer::TaskStderr { data, .. } => {
                stderr.push_str(&pc_task_text_chunk(&data));
            }
            AgentToServer::TaskExit { code, .. } => {
                exit_code = code;
                break;
            }
            AgentToServer::TaskError { message, .. } => return Err(anyhow!(message)),
            _ => {}
        }
    }

    if exit_code != Some(0) {
        if exit_code == Some(2) {
            return Ok(None);
        }
        return Err(anyhow!(
            "PC APK 查找命令退出 {:?}: {}",
            exit_code,
            stderr.trim()
        ));
    }

    let Some(relay) = parse_pc_apk_relay_output(&output)? else {
        return Ok(None);
    };
    let artifact_dir = artifact_workspace.join("artifacts");
    tokio::fs::create_dir_all(&artifact_dir).await?;
    let sha256 = format!("{:x}", Sha256::digest(&relay.apk_bytes));
    let size_bytes = relay.apk_bytes.len() as i64;
    let artifact_name = unique_pc_apk_artifact_name(&relay.file_name, &sha256);
    let artifact_path = artifact_dir.join(artifact_name);
    tokio::fs::write(&artifact_path, relay.apk_bytes).await?;
    Ok(Some(SyncedPcApk {
        path: artifact_path,
        file_name: relay.file_name,
        sha256,
        size_bytes,
        metadata: relay.metadata,
    }))
}

fn pc_task_text_chunk(data: &str) -> String {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

    let trimmed = data.trim_end_matches(['\r', '\n']);
    if !trimmed.is_empty() {
        if let Ok(bytes) = B64.decode(trimmed) {
            if let Ok(decoded) = String::from_utf8(bytes) {
                if decoded.contains("ELON_APK_")
                    || decoded.ends_with('\n')
                    || decoded.ends_with('\r')
                {
                    return decoded;
                }
            }
        }
    }
    data.to_string()
}

fn parse_pc_apk_relay_output(output: &str) -> Result<Option<PcApkRelayOutput>> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

    let Some(begin_index) = output.find("ELON_APK_BASE64_BEGIN") else {
        return Ok(None);
    };
    let Some(end_index) = output.find("ELON_APK_BASE64_END") else {
        return Ok(None);
    };
    if end_index <= begin_index {
        return Ok(None);
    }

    let filename = output
        .lines()
        .find_map(|line| line.trim().strip_prefix("ELON_APK_NAME:"))
        .map(safe_pc_apk_filename)
        .unwrap_or_else(|| "ElonSpeed-latest.apk".to_string());
    let payload = output[begin_index + "ELON_APK_BASE64_BEGIN".len()..end_index]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<String>();
    if payload.is_empty() {
        return Ok(None);
    }
    let apk_bytes = B64.decode(payload)?;
    Ok(Some(PcApkRelayOutput {
        file_name: filename,
        apk_bytes,
        metadata: SyncedPcApkMetadata {
            package_name: relay_metadata_line(output, "ELON_APK_PACKAGE:"),
            version_name: relay_metadata_line(output, "ELON_APK_VERSION_NAME:"),
            version_code: relay_metadata_line(output, "ELON_APK_VERSION_CODE:")
                .and_then(|value| value.parse::<i64>().ok()),
            build_started_at: relay_metadata_line(output, "ELON_APK_BUILD_STARTED_AT:"),
            source_git_sha: relay_metadata_line(output, "ELON_APK_GIT_SHA:"),
            apk_modified_at: relay_metadata_line(output, "ELON_APK_MODIFIED_AT:"),
        },
    }))
}

fn relay_metadata_line(output: &str, prefix: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let value = line.trim().strip_prefix(prefix)?.trim();
        if value.is_empty() || value.len() > 512 {
            return None;
        }
        let cleaned = value
            .chars()
            .filter(|ch| !ch.is_control())
            .collect::<String>();
        (!cleaned.is_empty()).then_some(cleaned)
    })
}

fn unique_pc_apk_artifact_name(file_name: &str, sha256: &str) -> String {
    let prefix = sha256.chars().take(12).collect::<String>();
    format!("{prefix}-{}", safe_pc_apk_filename(file_name))
}

fn synced_pc_apk_metadata_json(metadata: &SyncedPcApkMetadata) -> Option<String> {
    let value = serde_json::json!({
        "source": "pc_node_apk_sync",
        "package_name": metadata.package_name.as_deref(),
        "version_name": metadata.version_name.as_deref(),
        "version_code": metadata.version_code,
        "build_started_at": metadata.build_started_at.as_deref(),
        "source_git_sha": metadata.source_git_sha.as_deref(),
        "apk_modified_at": metadata.apk_modified_at.as_deref(),
    });
    Some(value.to_string()).filter(|json| json != "{}")
}

pub(crate) fn safe_pc_apk_filename(raw: &str) -> String {
    let basename = raw
        .rsplit(|ch| ch == '/' || ch == '\\')
        .next()
        .unwrap_or(raw)
        .trim();
    let safe = basename
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
        .collect::<String>();
    if safe.to_ascii_lowercase().ends_with(".apk") && !safe.is_empty() {
        safe
    } else {
        "ElonSpeed-latest.apk".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_pc_apk_relay_output, pc_task_text_chunk, project_id_from_download_base,
        unique_pc_apk_artifact_name,
    };
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

    #[test]
    fn parses_project_download_base() {
        assert_eq!(
            project_id_from_download_base("https://example.test/api/projects/prj_123/download"),
            Some("prj_123")
        );
    }

    #[test]
    fn parses_user_project_download_base() {
        assert_eq!(
            project_id_from_download_base(
                "https://example.test/api/user/usr_1/projects/prj_abc/download?token=t"
            ),
            Some("prj_abc")
        );
    }

    #[test]
    fn parses_raw_pc_agent_relay_output() {
        let apk_bytes = b"fake apk bytes";
        let payload = B64.encode(apk_bytes);
        let output = format!(
            "ELON_APK_NAME:app-debug.apk\nELON_APK_PACKAGE:com.example.app\nELON_APK_VERSION_CODE:42\nELON_APK_VERSION_NAME:1.2.3\nELON_APK_GIT_SHA:abc1234\nELON_APK_BASE64_BEGIN\n{}\n{}\nELON_APK_BASE64_END\n",
            &payload[..8],
            &payload[8..]
        );

        let relay = parse_pc_apk_relay_output(&output)
            .expect("relay output should parse")
            .expect("apk should be present");

        assert_eq!(relay.file_name, "app-debug.apk");
        assert_eq!(relay.apk_bytes, apk_bytes);
        assert_eq!(
            relay.metadata.package_name.as_deref(),
            Some("com.example.app")
        );
        assert_eq!(relay.metadata.version_code, Some(42));
        assert_eq!(relay.metadata.version_name.as_deref(), Some("1.2.3"));
        assert_eq!(relay.metadata.source_git_sha.as_deref(), Some("abc1234"));
    }

    #[test]
    fn unique_artifact_name_keeps_original_download_name_safe() {
        assert_eq!(
            unique_pc_apk_artifact_name("..\\app-debug.apk", "abcdef1234567890"),
            "abcdef123456-app-debug.apk"
        );
    }

    #[test]
    fn pc_task_text_chunk_keeps_raw_apk_payload_line() {
        let raw_payload = B64.encode([0, 159, 146, 150, 255, 0, 1, 2]);

        assert_eq!(
            pc_task_text_chunk(&(raw_payload.clone() + "\n")),
            raw_payload + "\n"
        );
    }

    #[test]
    fn pc_task_text_chunk_decodes_legacy_base64_text_line() {
        let legacy = B64.encode("ELON_APK_BASE64_BEGIN\n");

        assert_eq!(pc_task_text_chunk(&legacy), "ELON_APK_BASE64_BEGIN\n");
    }
}
