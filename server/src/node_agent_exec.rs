//! PC 节点 Exec 命令执行（流式 stdout/stderr/exit）。
//! 从 node_agent_main.rs 拆分，保持行为不变。

use tokio_tungstenite::tungstenite::Message;
use tracing::warn;
use homecli_proto::AgentToServer;

use super::ws_text;


/// 执行 Exec：运行任意命令，流式返回 TaskStdout/TaskStderr/TaskExit。
pub async fn run_exec(
    task_id: String,
    cli: String,
    args: Vec<String>,
    cwd: String,
    env_vars: Vec<(String, String)>,
    project_context: Option<homecli_proto::CliProjectContext>,
    data_paths: Option<elon_pc_dev_runtime::NodeDataPaths>,
    out_tx: tokio::sync::mpsc::UnboundedSender<Message>,
) {
    use tokio::io::AsyncBufReadExt;

    let mut build_run = if let Some(project_context) = project_context.as_ref() {
        let Some(data_paths) = data_paths.as_ref() else {
            let _ = out_tx.send(ws_text(&AgentToServer::TaskError {
                task_id,
                message: "PC 节点尚未配置统一数据根，已阻止项目 Exec 回落到系统盘".into(),
            }));
            return;
        };
        match crate::node_agent_build_runtime::prepare_run(
            data_paths,
            crate::node_agent_build_runtime::BuildRunRequest {
                task_id: &task_id,
                project_id: &project_context.project_id,
                cwd: Some(std::path::Path::new(&cwd)),
            },
        ) {
            Ok(run) => Some(run),
            Err(error) => {
                let _ = out_tx.send(ws_text(&AgentToServer::TaskError {
                    task_id,
                    message: format!("PC 节点构建环境门禁失败: {error:#}"),
                }));
                return;
            }
        }
    } else {
        None
    };
    let mut cmd = tokio::process::Command::new(&cli);
    cmd.args(&args).current_dir(&cwd);
    for (k, v) in &env_vars {
        cmd.env(k, v);
    }
    if let Some(run) = build_run.as_ref() {
        run.environment().apply_tokio(&mut cmd);
    }
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdin(std::process::Stdio::null());
    hide_tokio_command_window(&mut cmd);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = out_tx.send(ws_text(&AgentToServer::TaskError {
                task_id,
                message: format!("无法启动 {}: {}", cli, e),
            }));
            return;
        }
    };

    let pid = child.id().unwrap_or(0);
    let _ = out_tx.send(ws_text(&AgentToServer::TaskStarted {
        task_id: task_id.clone(),
        pid,
    }));

    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let mut stdout_lines = tokio::io::BufReader::new(stdout).lines();
    // stderr 字节级读取，避免 Windows GBK 编码触发 UTF-8 错误
    let (stderr_tx2, mut stderr_rx2) = tokio::sync::mpsc::unbounded_channel::<Option<String>>();
    {
        let tx = stderr_tx2.clone();
        let task_id2 = task_id.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut reader = tokio::io::BufReader::new(stderr);
            let mut buf = Vec::new();
            loop {
                buf.clear();
                match reader.read_until(b'\n', &mut buf).await {
                    Ok(0) | Err(_) => {
                        let _ = tx.send(None);
                        break;
                    }
                    Ok(_) => {
                        while matches!(buf.last(), Some(&b'\n') | Some(&b'\r')) {
                            buf.pop();
                        }
                        let _ = tx.send(Some(String::from_utf8_lossy(&buf).into_owned()));
                    }
                }
            }
            drop(task_id2); // 保持 task_id2 活跃直到 stderr 读完
        });
    }
    let mut stdout_done = false;
    let mut stderr_done = false;

    while !stdout_done || !stderr_done {
        tokio::select! {
            line = stdout_lines.next_line(), if !stdout_done => match line {
                Ok(Some(l)) => { let _ = out_tx.send(ws_text(&AgentToServer::TaskStdout { task_id: task_id.clone(), data: l + "\n" })); }
                Ok(None) => { stdout_done = true; }
                Err(e) => { warn!("stdout err: {e}"); stdout_done = true; }
            },
            opt = stderr_rx2.recv(), if !stderr_done => match opt {
                Some(Some(l)) => { let _ = out_tx.send(ws_text(&AgentToServer::TaskStderr { task_id: task_id.clone(), data: l + "\n" })); }
                Some(None) | None => { stderr_done = true; }
            },
        }
    }

    let code = child.wait().await.ok().and_then(|s| s.code());
    if let Some(run) = build_run.as_mut() {
        run.finish(code == Some(0));
    }
    drop(build_run);
    let _ = out_tx.send(ws_text(&AgentToServer::TaskExit { task_id, code }));
}

pub fn hide_tokio_command_window(_command: &mut tokio::process::Command) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        _command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    }
}

