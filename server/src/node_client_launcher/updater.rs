use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

use super::{env_file, paths, process, DEFAULT_BASE_URL};

#[derive(Debug, Deserialize)]
struct VersionInfo {
    #[serde(default, rename = "gitSha")]
    git_sha: String,
    #[serde(default, rename = "downloadUrl")]
    download_url: String,
}

pub(crate) fn update_agent_if_needed(install_dir: &Path) -> Result<()> {
    if let Err(error) = try_update_agent_if_needed(install_dir) {
        eprintln!("自动更新检查失败，继续使用本地版本: {error:#}");
    }
    Ok(())
}

fn try_update_agent_if_needed(install_dir: &Path) -> Result<()> {
    let env_values = env_file::read_env_file(&paths::env_file(install_dir))?;
    if env_values
        .get("NODE_AGENT_AUTO_UPDATE")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no" | "off"
            )
        })
        .unwrap_or(false)
    {
        return Ok(());
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
        return Ok(());
    }

    let local_sha = read_local_git_sha(&paths::version_file(install_dir)).unwrap_or_default();
    if local_sha == remote.git_sha {
        return Ok(());
    }

    let download_url = if remote.download_url.trim().is_empty() {
        format!(
            "{}/api/node-agent/download/windows",
            base_url.trim_end_matches('/')
        )
    } else {
        remote.download_url.clone()
    };
    let tmp = paths::agent_exe(install_dir).with_extension("exe.new");
    let bytes = reqwest::blocking::get(&download_url)
        .with_context(|| format!("无法下载 {download_url}"))?
        .error_for_status()
        .with_context(|| format!("下载接口返回错误 {download_url}"))?
        .bytes()
        .context("无法读取下载内容")?;
    if bytes.len() < 1024 * 1024 {
        anyhow::bail!("下载的节点程序过小，疑似异常响应");
    }
    std::fs::write(&tmp, &bytes).with_context(|| format!("无法写入 {}", tmp.display()))?;

    process::stop_agent();
    let agent_path = paths::agent_exe(install_dir);
    let _ = std::fs::remove_file(&agent_path);
    std::fs::rename(&tmp, &agent_path).context("替换内部节点程序失败")?;
    std::fs::write(paths::version_file(install_dir), remote_text).context("写入版本文件失败")?;
    Ok(())
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
