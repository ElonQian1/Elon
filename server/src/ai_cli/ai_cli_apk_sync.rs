use anyhow::{anyhow, Result};
use homecli_proto::AgentToServer;
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;

use super::{looks_like_android_task, AiCliRequestMode};
use crate::{
    tools,
    types::{AppState, WsMessage},
};

pub(crate) fn pc_apk_probe_since(request_mode: AiCliRequestMode, cwd: Option<&str>) -> Option<u64> {
    if request_mode != AiCliRequestMode::Execute || cwd.is_none() {
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
    if request_mode != AiCliRequestMode::Execute {
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
    )
    .await
    {
        Ok(Some(_path)) => Some(tools::stable_apk_url(download_base)),
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
) -> Result<Option<std::path::PathBuf>> {
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

    let freshness_filter = fresh_after_unix_secs
        .map(|secs| {
            format!(
                "$minModifiedUtc = [DateTimeOffset]::FromUnixTimeSeconds({secs}).UtcDateTime\n$files = @($files | Where-Object {{ $_.LastWriteTimeUtc -ge $minModifiedUtc }})"
            )
        })
        .unwrap_or_default();
    let script = r#"
$ErrorActionPreference = 'Stop'
$roots = @(
  (Join-Path (Get-Location) 'app\build\outputs\apk'),
  (Join-Path (Get-Location) 'android\app\build\outputs\apk'),
  (Join-Path (Get-Location) 'build'),
  (Join-Path (Get-Location) 'artifacts')
)
$files = @()
foreach ($root in $roots) {
  if (Test-Path -LiteralPath $root) {
    $files += Get-ChildItem -LiteralPath $root -Recurse -Filter *.apk -File -ErrorAction SilentlyContinue
  }
}
__ELON_FRESHNESS_FILTER__
$apk = $files | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
if (-not $apk) { exit 2 }
if ($apk.Length -gt 104857600) { Write-Error 'APK too large to relay'; exit 3 }
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Write-Output ('ELON_APK_NAME:' + $apk.Name)
Write-Output 'ELON_APK_BASE64_BEGIN'
Write-Output ([Convert]::ToBase64String([System.IO.File]::ReadAllBytes($apk.FullName)))
Write-Output 'ELON_APK_BASE64_END'
"#
    .replace("__ELON_FRESHNESS_FILTER__", &freshness_filter);
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
                script.to_string(),
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
                if let Ok(bytes) = B64.decode(&data) {
                    output.push_str(&String::from_utf8_lossy(&bytes));
                }
            }
            AgentToServer::TaskStderr { data, .. } => {
                if let Ok(bytes) = B64.decode(&data) {
                    stderr.push_str(&String::from_utf8_lossy(&bytes));
                }
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
    tokio::fs::write(&artifact_path, apk_bytes).await?;
    Ok(Some(artifact_path))
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
    use super::project_id_from_download_base;

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
}
