//! Revalidate runtime ownership immediately before entering update install.

use super::*;

pub(super) fn fresh_runtime_handle_task_ids(
    install_dir: &Path,
) -> Result<std::collections::HashSet<String>> {
    let env_values = env_file::read_env_file(&paths::env_file(install_dir))?;
    let port = process::admin_port_from_env_values(&env_values);
    let response = reqwest::blocking::Client::builder()
        .no_proxy()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(4))
        .build()
        .context("无法创建本机 runtime handle 探针")?
        .get(format!("http://127.0.0.1:{port}/api/status"))
        .send();
    let response = match response {
        Ok(response) => response,
        Err(_) if !process::agent_runtime_running(install_dir) => {
            return Ok(std::collections::HashSet::new())
        }
        Err(error) => {
            return Err(error)
                .context("已安装 runtime 仍在运行，但无法重新验证活动 handle；拒绝进入安装窗口")
        }
    };
    anyhow::ensure!(
        response.status().is_success(),
        "已安装 runtime 状态探针返回 {}，拒绝进入安装窗口",
        response.status()
    );
    let status: serde_json::Value = response
        .json()
        .context("已安装 runtime 状态不是合法 JSON，拒绝进入安装窗口")?;
    runtime_handle_task_ids_from_status(&status)
}

pub(super) fn runtime_handle_task_ids_from_status(
    status: &serde_json::Value,
) -> Result<std::collections::HashSet<String>> {
    anyhow::ensure!(
        status
            .get("local_admin_token_header")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty()),
        "本机状态响应不属于一龙节点 runtime"
    );
    let reported_count = status
        .get("active_cli_prompt_count")
        .and_then(serde_json::Value::as_u64)
        .context("runtime 状态缺少活动 prompt 数量")? as usize;
    let prompt_task_ids = status
        .get("active_cli_prompt_task_ids")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut task_ids = prompt_task_ids
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<std::collections::HashSet<_>>();
    task_ids.extend(
        status
            .get("active_task_runtime")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.get("task_id"))
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
    );
    anyhow::ensure!(
        prompt_task_ids.len() >= reported_count && task_ids.len() >= reported_count,
        "runtime 报告 {reported_count} 个活动 handle，但只提供 {} 个可验证任务身份",
        task_ids.len()
    );
    Ok(task_ids)
}

pub(super) fn restart_installed_runtime_and_watchdog(install_dir: &Path) -> Result<()> {
    let port = process::start_background(install_dir)?;
    watchdog::ensure_running(install_dir)?;
    process::verify_background_ready(port)?;
    log_file::record_event(
        install_dir,
        "update_installed_runtime_watchdog_recovered",
        true,
        "runtime and watchdog started from the installed client path after replacement",
    );
    Ok(())
}
