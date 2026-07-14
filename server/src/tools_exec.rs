use anyhow::{anyhow, Result};
use std::sync::Arc;
use tracing::info;

use crate::types::AppState;

/// PC 端 elon 项目路径（ELON_SELF_PC_PATH 环境变量，默认 Windows 路径）
pub fn elon_pc_project_path() -> String {
    std::env::var("ELON_SELF_PC_PATH")
        .unwrap_or_else(|_| r"D:\rust\active-projects\elon cli".into())
}

/// 通过 homecli PC 代理执行命令，收集完整输出并将进度实时推送给 APK 客户端
pub async fn exec_via_agent(
    state: &Arc<AppState>,
    cli: &str,
    args: Vec<String>,
    cwd: &str,
    project_context: Option<homecli_proto::CliProjectContext>,
    progress_tx: Option<&tokio::sync::mpsc::UnboundedSender<String>>,
) -> Result<String> {
    use crate::types::WsMessage;
    use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
    use homecli_proto::AgentToServer;

    let agents = state.agent_manager.list().await;
    let agent_id = agents
        .first()
        .ok_or_else(|| anyhow!("没有可用的 PC agent，请确认 homecli 已启动并连接到服务器"))?
        .agent_id
        .clone();

    let (_task_id, mut rx) = state
        .agent_manager
        .dispatch_with_project_context(
            &agent_id,
            cli.to_string(),
            args,
            cwd.to_string(),
            vec![],
            project_context,
        )
        .await?;

    info!(%agent_id, %cli, %cwd, "exec_via_agent: dispatched");

    let mut output_bytes = Vec::<u8>::new();
    let mut exit_code: Option<i32> = None;

    while let Some(msg) = rx.recv().await {
        match msg {
            AgentToServer::TaskStarted { pid, .. } => {
                info!(%agent_id, %pid, "exec_via_agent: task started");
                if let Some(tx) = progress_tx {
                    let _ = tx.send(
                        WsMessage::progress(format!("[PC agent] 任务启动 pid={}", pid)).to_json(),
                    );
                }
            }
            AgentToServer::TaskStdout { data, .. } => {
                if let Ok(bytes) = B64.decode(&data) {
                    if let Some(tx) = progress_tx {
                        let s = String::from_utf8_lossy(&bytes);
                        for line in s.lines().filter(|l| !l.trim().is_empty()) {
                            let _ = tx.send(WsMessage::progress(line.to_string()).to_json());
                        }
                    }
                    output_bytes.extend_from_slice(&bytes);
                }
            }
            AgentToServer::TaskStderr { data, .. } => {
                if let Ok(bytes) = B64.decode(&data) {
                    if let Some(tx) = progress_tx {
                        let s = String::from_utf8_lossy(&bytes);
                        for line in s.lines().filter(|l| !l.trim().is_empty()) {
                            let _ = tx
                                .send(WsMessage::progress(format!("[stderr] {}", line)).to_json());
                        }
                    }
                    output_bytes.extend_from_slice(&bytes);
                }
            }
            AgentToServer::TaskExit { code, .. } => {
                exit_code = code;
                break;
            }
            AgentToServer::TaskError { message, .. } => {
                return Err(anyhow!("PC agent 任务失败: {}", message));
            }
            _ => {}
        }
    }

    let output = String::from_utf8_lossy(&output_bytes).to_string();
    match exit_code {
        Some(0) | None => Ok(output),
        Some(code) => Err(anyhow!(
            "PC agent 退出码 {}\n{}",
            code,
            &output[..output.len().min(2000)]
        )),
    }
}

/// 通过 homecli PC 代理触发项目构建脚本（android/rust），失败时调用者可回退到本地构建
pub async fn build_project_via_agent(
    state: &Arc<AppState>,
    target: &str,
    changelog: &str,
    progress_tx: Option<&tokio::sync::mpsc::UnboundedSender<String>>,
) -> Result<String> {
    let pc_cwd = elon_pc_project_path();
    let (cli, args): (&str, Vec<String>) = match target {
        "android" => (
            "pwsh",
            vec![
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                r"scripts\publish-apk.ps1".into(),
                "-Changelog".into(),
                changelog.to_string(),
            ],
        ),
        "rust" => (
            "pwsh",
            vec![
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                r"scripts\publish-server.ps1".into(),
            ],
        ),
        _ => {
            return Err(anyhow!(
                "PC agent 不支持构建目标: {}（支持: android / rust）",
                target
            ))
        }
    };
    let output = exec_via_agent(
        state,
        cli,
        args,
        &pc_cwd,
        Some(homecli_proto::CliProjectContext {
            project_id: "elon-platform".to_string(),
            conversation_id: format!("platform-build-{target}"),
            runtime_permission: Some("project_write".to_string()),
        }),
        progress_tx,
    )
    .await?;
    if target == "android" {
        return Ok(format!(
            "android 构建成功（PC agent）\n##APK_FILE:ElonSpeed-latest.apk\n\n{}",
            &output[..output.len().min(500)]
        ));
    }
    Ok(format!(
        "{} 构建成功（PC agent）\n{}",
        target,
        &output[..output.len().min(500)]
    ))
}
