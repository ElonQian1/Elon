// server/src/node_client_launcher/updater.rs

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

use super::{command as launcher_command, env_file, paths, process, DEFAULT_BASE_URL};

#[derive(Debug, Deserialize)]
struct VersionInfo {
    #[serde(default, rename = "gitSha")]
    git_sha: String,
    #[serde(default, rename = "downloadUrl")]
    download_url: String,
}

pub(crate) fn update_client_if_needed(install_dir: &Path) -> Result<bool> {
    match try_update_client_if_needed(install_dir) {
        Ok(scheduled_restart) => Ok(scheduled_restart),
        Err(error) => {
            eprintln!("自动更新检查失败，继续使用本地版本: {error:#}");
            Ok(false)
        }
    }
}

fn try_update_client_if_needed(install_dir: &Path) -> Result<bool> {
    let env_values = env_file::read_env_file(&paths::env_file(install_dir))?;
    if auto_update_disabled(&env_values) {
        return Ok(false);
    }

    let base_url = update_base_url(&env_values);
    let version_url = format!("{}/api/node-agent/version", base_url.trim_end_matches('/'));
    let remote_text = reqwest::blocking::get(&version_url)
        .with_context(|| format!("无法请求 {version_url}"))?
        .error_for_status()
        .with_context(|| format!("版本接口返回错误 {version_url}"))?
        .text()
        .context("无法读取远程版本内容")?;
    let remote_text = remote_text.trim_start_matches('\u{feff}').to_string();
    let remote: VersionInfo =
        serde_json::from_str(&remote_text).context("远程版本内容不是合法 JSON")?;
    if remote.git_sha.trim().is_empty() {
        return Ok(false);
    }

    let version_file = paths::version_file(install_dir);
    let local_sha = read_local_git_sha(&version_file).unwrap_or_default();
    if local_sha == remote.git_sha {
        return Ok(false);
    }

    let download_url = if remote.download_url.trim().is_empty() {
        format!(
            "{}/api/node-agent/download/windows",
            base_url.trim_end_matches('/')
        )
    } else {
        remote.download_url.clone()
    };

    let internal_dir = paths::internal_dir(install_dir);
    std::fs::create_dir_all(&internal_dir)
        .with_context(|| format!("无法创建内部目录 {}", internal_dir.display()))?;
    let tmp_exe = internal_dir.join("一龙PC节点.exe.new");
    let tmp_version = internal_dir.join("node-agent-version.json.new");
    let bytes = reqwest::blocking::get(&download_url)
        .with_context(|| format!("无法下载 {download_url}"))?
        .error_for_status()
        .with_context(|| format!("下载接口返回错误 {download_url}"))?
        .bytes()
        .context("无法读取下载内容")?;
    if bytes.len() < 1024 * 1024 {
        anyhow::bail!("下载的客户端程序过小，疑似异常响应");
    }
    std::fs::write(&tmp_exe, &bytes).with_context(|| format!("无法写入 {}", tmp_exe.display()))?;
    std::fs::write(&tmp_version, remote_text)
        .with_context(|| format!("无法写入 {}", tmp_version.display()))?;

    process::stop_agent();
    let client = paths::client_exe(install_dir);
    let uninstall = paths::uninstall_exe(install_dir);
    if running_from_path(&client) {
        schedule_self_replace(&tmp_exe, &client, &uninstall, &tmp_version, &version_file)?;
        Ok(true)
    } else {
        replace_client_files(&tmp_exe, &client, &uninstall, &tmp_version, &version_file)?;
        Ok(false)
    }
}

fn auto_update_disabled(env_values: &std::collections::HashMap<String, String>) -> bool {
    env_values
        .get("NODE_AGENT_AUTO_UPDATE")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no" | "off"
            )
        })
        .unwrap_or(false)
}

fn replace_client_files(
    tmp_exe: &Path,
    client: &Path,
    uninstall: &Path,
    tmp_version: &Path,
    version_file: &Path,
) -> Result<()> {
    let _ = std::fs::remove_file(client);
    std::fs::rename(tmp_exe, client).context("替换客户端主程序失败")?;
    std::fs::copy(client, uninstall).context("同步卸载程序失败")?;
    let _ = std::fs::remove_file(version_file);
    std::fs::rename(tmp_version, version_file).context("替换版本信息失败")?;
    Ok(())
}

fn running_from_path(path: &Path) -> bool {
    std::env::current_exe()
        .ok()
        .map(|current| same_path(&current, path))
        .unwrap_or(false)
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = std::fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = std::fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    left == right
}

fn schedule_self_replace(
    tmp_exe: &Path,
    client: &Path,
    uninstall: &Path,
    tmp_version: &Path,
    version_file: &Path,
) -> Result<()> {
    #[cfg(windows)]
    {
        let script = self_replace_script(tmp_exe, client, uninstall, tmp_version, version_file);
        // 自替换必须等当前进程退出，脚本保留但入口和重启都强制隐藏窗口。
        let mut cmd = launcher_command::powershell_hidden_command(&script);
        launcher_command::spawn_hidden(&mut cmd).context("无法安排客户端自更新")?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        replace_client_files(tmp_exe, client, uninstall, tmp_version, version_file)
    }
}

#[cfg(windows)]
fn self_replace_script(
    tmp_exe: &Path,
    client: &Path,
    uninstall: &Path,
    tmp_version: &Path,
    version_file: &Path,
) -> String {
    format!(
        r#"
$ErrorActionPreference = 'Stop'
$pidToWait = {pid}
$tmpExe = '{tmp_exe}'
$client = '{client}'
$uninstall = '{uninstall}'
$tmpVersion = '{tmp_version}'
$versionFile = '{version_file}'
Wait-Process -Id $pidToWait -ErrorAction SilentlyContinue
Move-Item -LiteralPath $tmpExe -Destination $client -Force
Copy-Item -LiteralPath $client -Destination $uninstall -Force
Move-Item -LiteralPath $tmpVersion -Destination $versionFile -Force
Start-Process -FilePath $client -WindowStyle Hidden
"#,
        pid = std::process::id(),
        tmp_exe = launcher_command::ps_single_quote(&tmp_exe.to_string_lossy()),
        client = launcher_command::ps_single_quote(&client.to_string_lossy()),
        uninstall = launcher_command::ps_single_quote(&uninstall.to_string_lossy()),
        tmp_version = launcher_command::ps_single_quote(&tmp_version.to_string_lossy()),
        version_file = launcher_command::ps_single_quote(&version_file.to_string_lossy())
    )
}

fn update_base_url(env_values: &std::collections::HashMap<String, String>) -> String {
    env_values
        .get("NODE_AGENT_UPDATE_BASE_URL")
        .cloned()
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
}

fn read_local_git_sha(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value
        .get("gitSha")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn self_replace_script_stops_on_failure_before_restart() {
        use std::path::Path;

        let script = super::self_replace_script(
            Path::new(r"C:\ElonNode\_internal\一龙PC节点.exe.new"),
            Path::new(r"C:\ElonNode\一龙PC节点.exe"),
            Path::new(r"C:\ElonNode\卸载一龙PC节点.exe"),
            Path::new(r"C:\ElonNode\_internal\node-agent-version.json.new"),
            Path::new(r"C:\ElonNode\_internal\node-agent-version.json"),
        );

        assert!(script.contains("$ErrorActionPreference = 'Stop'"));
        assert!(script.contains("Start-Process -FilePath $client -WindowStyle Hidden"));
        assert!(
            script.find("Move-Item -LiteralPath $tmpExe").unwrap()
                < script.find("Start-Process -FilePath $client").unwrap()
        );
    }
}
