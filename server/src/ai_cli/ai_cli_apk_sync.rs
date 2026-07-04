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
    let fresh_after_unix_secs = if explicit_apk_sync {
        None
    } else {
        apk_sync_probe_since
    };
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
            if let Some(apk_url) = latest_project_release_apk_url(state, download_base) {
                if explicit_apk_sync {
                    let _ = tx.send(
                        WsMessage::progress(
                            "本轮 PC 工作区没有发现新的 APK，已复用项目空间最新安装包入口。",
                        )
                        .to_json(),
                    );
                }
                return Some(apk_url);
            }
            if explicit_apk_sync {
                let _ = tx.send(
                    WsMessage::progress("本轮 PC 工作区没有发现 APK；不会生成安装按钮链接。")
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
    if let Err(error) =
        state
            .store
            .create_project_release(crate::store::project_releases::ProjectReleaseWrite {
                id: None,
                project_id,
                task_id: None,
                uploaded_by: None,
                version_name: Some("PC node debug build"),
                channel: Some("pc_node"),
                status: Some("published"),
                apk_url,
                file_name: &apk.file_name,
                file_path: Some(file_path.as_ref()),
                sha256: Some(&apk.sha256),
                size_bytes: Some(apk.size_bytes),
                changelog: Some("Synced from PC node after AI development task"),
            })
    {
        tracing::warn!(
            project_id = %project_id,
            error = %error,
            "failed to register synced PC APK release"
        );
    }
}

fn latest_project_release_apk_url(state: &AppState, download_base: &str) -> Option<String> {
    let project_id = project_id_from_download_base(download_base)?;
    if project_id.is_empty() {
        return None;
    }
    match state.store.latest_project_apk_url(project_id) {
        Ok(Some(apk_url)) => Some(apk_url),
        Ok(None) => None,
        Err(error) => {
            tracing::warn!(
                project_id = %project_id,
                error = %error,
                "failed to read latest project APK URL after PC sync miss"
            );
            None
        }
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
    fresh_after_unix_secs: Option<u64>,
    build_if_missing: bool,
) -> Result<Option<SyncedPcApk>> {
    let command =
        pc_apk_sync_loader_command(&state.public_url, fresh_after_unix_secs, build_if_missing);
    let (_task_id, mut rx) = state
        .agent_manager
        .dispatch(
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

    let Some((filename, apk_bytes)) = parse_pc_apk_relay_output(&output)? else {
        return Ok(None);
    };
    let artifact_dir = artifact_workspace.join("artifacts");
    tokio::fs::create_dir_all(&artifact_dir).await?;
    let artifact_path = artifact_dir.join(filename);
    let sha256 = format!("{:x}", Sha256::digest(&apk_bytes));
    let size_bytes = apk_bytes.len() as i64;
    tokio::fs::write(&artifact_path, apk_bytes).await?;
    let file_name = artifact_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("ElonSpeed-latest.apk")
        .to_string();
    Ok(Some(SyncedPcApk {
        path: artifact_path,
        file_name,
        sha256,
        size_bytes,
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

fn parse_pc_apk_relay_output(output: &str) -> Result<Option<(String, Vec<u8>)>> {
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
    Ok(Some((filename, apk_bytes)))
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
    use super::{parse_pc_apk_relay_output, pc_task_text_chunk, project_id_from_download_base};
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
            "ELON_APK_NAME:app-debug.apk\nELON_APK_BASE64_BEGIN\n{}\n{}\nELON_APK_BASE64_END\n",
            &payload[..8],
            &payload[8..]
        );

        let (filename, parsed) = parse_pc_apk_relay_output(&output)
            .expect("relay output should parse")
            .expect("apk should be present");

        assert_eq!(filename, "app-debug.apk");
        assert_eq!(parsed, apk_bytes);
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
