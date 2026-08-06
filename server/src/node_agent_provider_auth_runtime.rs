//! Official provider login runtimes owned by the local CLI processes.
//!
//! Codex uses `codex app-server`; Gemini uses the stable ACP v1 stdio
//! handshake. This module never parses or exports provider credential files.

use anyhow::{anyhow, Context, Result};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::Path,
    process::Stdio,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::{watch, Mutex, RwLock},
    time::{timeout, Duration, Instant},
};
use uuid::Uuid;

pub(crate) use crate::node_agent_provider_auth_attempt::ProviderLoginAttempt;
use crate::node_agent_provider_auth_attempt_store::ProviderAuthAttemptStore;
use crate::node_agent_provider_auth_protocol::{
    client_info, codex_login_instructions, select_gemini_auth_method,
};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);
const LOGIN_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const MAX_RPC_LINE_BYTES: usize = 256 * 1024;
const ATTEMPT_RETENTION_MS: u64 = 24 * 60 * 60 * 1000;

struct AttemptControl {
    view: Arc<RwLock<ProviderLoginAttempt>>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    cancel_tx: watch::Sender<bool>,
    upstream_login_id: Option<String>,
}

#[derive(Clone)]
pub(crate) struct ProviderAuthRuntime {
    attempts: Arc<RwLock<HashMap<String, Arc<AttemptControl>>>>,
    journal: ProviderAuthAttemptStore,
    start_gate: Arc<Mutex<()>>,
}

impl Default for ProviderAuthRuntime {
    fn default() -> Self {
        Self::new(crate::node_agent_provider_auth_attempt_store::default_journal_path(None))
    }
}

impl ProviderAuthRuntime {
    pub(crate) fn new(path: std::path::PathBuf) -> Self {
        let (journal, recovered) = ProviderAuthAttemptStore::load(path);
        let mut attempts = HashMap::new();
        for view in recovered {
            let (cancel_tx, _) = watch::channel(false);
            attempts.insert(
                view.login_id.clone(),
                Arc::new(AttemptControl {
                    view: Arc::new(RwLock::new(view)),
                    stdin: Arc::new(Mutex::new(None)),
                    cancel_tx,
                    upstream_login_id: None,
                }),
            );
        }
        Self {
            attempts: Arc::new(RwLock::new(attempts)),
            journal,
            start_gate: Arc::new(Mutex::new(())),
        }
    }

    pub(crate) async fn latest(&self, provider_id: &str) -> Option<ProviderLoginAttempt> {
        self.prune().await;
        let controls = self
            .attempts
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let mut views = Vec::new();
        for control in controls {
            let view = control.view.read().await.clone();
            if view.provider_id == provider_id {
                views.push(view);
            }
        }
        views.into_iter().max_by_key(|view| view.started_at_ms)
    }

    pub(crate) async fn get(
        &self,
        provider_id: &str,
        login_id: &str,
    ) -> Option<ProviderLoginAttempt> {
        let control = self.attempts.read().await.get(login_id).cloned()?;
        let view = control.view.read().await.clone();
        (view.provider_id == provider_id).then_some(view)
    }

    pub(crate) async fn start_codex_login(
        &self,
        program: &Path,
        flow: &str,
        request_id: Option<&str>,
    ) -> Result<ProviderLoginAttempt> {
        let _start = self.start_gate.lock().await;
        if let Some(existing) = self.idempotent_attempt("codex_cli", request_id).await {
            return Ok(existing);
        }
        if let Some(active) = self.active_attempt("codex_cli").await {
            return Ok(active);
        }
        let (mut child, stdin, mut reader) = spawn_rpc_process(program, &["app-server"]).await?;
        write_rpc(
            &stdin,
            &json!({
                "method": "initialize",
                "id": 1,
                "params": {"clientInfo": client_info()}
            }),
        )
        .await?;
        rpc_result(read_response(&mut reader, 1, STARTUP_TIMEOUT).await?)?;
        write_rpc(&stdin, &json!({"method": "initialized", "params": {}})).await?;
        let login_params = match flow {
            "browser" => json!({
                "type": "chatgpt",
                "useHostedLoginSuccessPage": true,
                "appBrand": "codex"
            }),
            "device_code" => json!({"type": "chatgptDeviceCode"}),
            _ => return Err(anyhow!("Codex 不支持登录流程 {flow}")),
        };
        write_rpc(
            &stdin,
            &json!({"method": "account/login/start", "id": 2, "params": login_params}),
        )
        .await?;
        let result = rpc_result(read_response(&mut reader, 2, STARTUP_TIMEOUT).await?)?;
        let instructions = codex_login_instructions(&result)?;
        let now = now_ms();
        let view = ProviderLoginAttempt {
            schema_version: 2,
            login_id: Uuid::new_v4().to_string(),
            provider_id: "codex_cli".to_string(),
            flow: flow.to_string(),
            state: "waiting_for_user".to_string(),
            request_id: request_id.map(ToOwned::to_owned),
            verification_url: instructions.verification_url,
            user_code: instructions.user_code,
            auth_url: instructions.auth_url,
            remote_compatible: flow == "device_code",
            recovered: false,
            error: None,
            error_code: None,
            started_at_ms: now,
            updated_at_ms: now,
        };
        self.register_attempt(
            view,
            instructions.upstream_login_id,
            child,
            stdin,
            reader,
            MonitorKind::Codex,
        )
        .await
    }

    pub(crate) async fn start_gemini_login(
        &self,
        program: &Path,
        request_id: Option<&str>,
    ) -> Result<ProviderLoginAttempt> {
        let _start = self.start_gate.lock().await;
        if let Some(existing) = self.idempotent_attempt("gemini_cli", request_id).await {
            return Ok(existing);
        }
        if let Some(active) = self.active_attempt("gemini_cli").await {
            return Ok(active);
        }
        let (child, stdin, mut reader) = spawn_rpc_process(program, &["--acp"]).await?;
        write_rpc(
            &stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "initialize",
                "id": 1,
                "params": {
                    "protocolVersion": 1,
                    "clientCapabilities": {},
                    "clientInfo": client_info()
                }
            }),
        )
        .await?;
        let initialize = rpc_result(read_response(&mut reader, 1, STARTUP_TIMEOUT).await?)?;
        let method_id = select_gemini_auth_method(&initialize)
            .context("Gemini ACP 没有公布 Google 账号登录方法")?;
        write_rpc(
            &stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": "authenticate",
                "id": 2,
                "params": {"methodId": method_id}
            }),
        )
        .await?;
        let now = now_ms();
        let view = ProviderLoginAttempt {
            schema_version: 2,
            login_id: Uuid::new_v4().to_string(),
            provider_id: "gemini_cli".to_string(),
            flow: "agent".to_string(),
            state: "waiting_for_user".to_string(),
            request_id: request_id.map(ToOwned::to_owned),
            verification_url: None,
            user_code: None,
            auth_url: None,
            remote_compatible: false,
            recovered: false,
            error: None,
            error_code: None,
            started_at_ms: now,
            updated_at_ms: now,
        };
        self.register_attempt(view, None, child, stdin, reader, MonitorKind::Gemini)
            .await
    }

    pub(crate) async fn start_claude_login(
        &self,
        program: &Path,
        request_id: Option<&str>,
    ) -> Result<ProviderLoginAttempt> {
        self.start_process_login(
            "claude_cli",
            "agent",
            program,
            &["auth", "login"],
            request_id,
        )
        .await
    }

    pub(crate) async fn start_copilot_login(
        &self,
        program: &Path,
        request_id: Option<&str>,
    ) -> Result<ProviderLoginAttempt> {
        self.start_process_login(
            "copilot_cli",
            "agent",
            program,
            &["login", "--web-flow"],
            request_id,
        )
        .await
    }

    pub(crate) async fn cancel(&self, provider_id: &str, login_id: &str) -> Result<()> {
        let control = self
            .attempts
            .read()
            .await
            .get(login_id)
            .cloned()
            .context("找不到该登录任务")?;
        if control.view.read().await.provider_id != provider_id {
            return Err(anyhow!("登录任务与厂商不匹配"));
        }
        if !control.view.read().await.is_active() {
            return Ok(());
        }
        let _ = control.cancel_tx.send(true);
        Ok(())
    }

    pub(crate) async fn logout_codex(&self, program: &Path) -> Result<()> {
        self.cancel_active("codex_cli").await;
        let (mut child, stdin, mut reader) = spawn_rpc_process(program, &["app-server"]).await?;
        write_rpc(
            &stdin,
            &json!({"method":"initialize","id":1,"params":{"clientInfo":client_info()}}),
        )
        .await?;
        rpc_result(read_response(&mut reader, 1, STARTUP_TIMEOUT).await?)?;
        write_rpc(&stdin, &json!({"method":"initialized","params":{}})).await?;
        write_rpc(&stdin, &json!({"method":"account/logout","id":2})).await?;
        rpc_result(read_response(&mut reader, 2, STARTUP_TIMEOUT).await?)?;
        stop_child(&mut child).await;
        Ok(())
    }

    pub(crate) async fn logout_gemini(&self, program: &Path) -> Result<()> {
        self.cancel_active("gemini_cli").await;
        let (mut child, stdin, mut reader) = spawn_rpc_process(program, &["--acp"]).await?;
        write_rpc(
            &stdin,
            &json!({
                "jsonrpc":"2.0","method":"initialize","id":1,
                "params":{"protocolVersion":1,"clientCapabilities":{},"clientInfo":client_info()}
            }),
        )
        .await?;
        let initialize = rpc_result(read_response(&mut reader, 1, STARTUP_TIMEOUT).await?)?;
        if initialize
            .pointer("/agentCapabilities/auth/logout")
            .is_none()
        {
            stop_child(&mut child).await;
            return Err(anyhow!(
                "当前 Gemini CLI 未公布 ACP logout 能力，请在 Gemini CLI 的 /auth 中退出"
            ));
        }
        write_rpc(
            &stdin,
            &json!({"jsonrpc":"2.0","method":"logout","id":2,"params":{}}),
        )
        .await?;
        rpc_result(read_response(&mut reader, 2, STARTUP_TIMEOUT).await?)?;
        stop_child(&mut child).await;
        Ok(())
    }

    pub(crate) async fn logout_claude(&self, program: &Path) -> Result<()> {
        self.cancel_active("claude_cli").await;
        run_provider_command(program, &["auth", "logout"]).await
    }

    async fn start_process_login(
        &self,
        provider_id: &str,
        flow: &str,
        program: &Path,
        args: &[&str],
        request_id: Option<&str>,
    ) -> Result<ProviderLoginAttempt> {
        let _start = self.start_gate.lock().await;
        if let Some(existing) = self.idempotent_attempt(provider_id, request_id).await {
            return Ok(existing);
        }
        if let Some(active) = self.active_attempt(provider_id).await {
            return Ok(active);
        }
        let mut std_command = elon_pc_dev_runtime::command_from_path(program);
        std_command.args(args);
        let mut command = Command::from(std_command);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let child = command
            .spawn()
            .with_context(|| format!("无法启动官方 CLI：{}", program.display()))?;
        let now = now_ms();
        let view = ProviderLoginAttempt {
            schema_version: 2,
            login_id: Uuid::new_v4().to_string(),
            provider_id: provider_id.to_string(),
            flow: flow.to_string(),
            state: "waiting_for_user".to_string(),
            request_id: request_id.map(ToOwned::to_owned),
            verification_url: None,
            user_code: None,
            auth_url: None,
            remote_compatible: false,
            recovered: false,
            error: None,
            error_code: None,
            started_at_ms: now,
            updated_at_ms: now,
        };
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let public = Arc::new(RwLock::new(view.clone()));
        self.attempts.write().await.insert(
            view.login_id.clone(),
            Arc::new(AttemptControl {
                view: public.clone(),
                stdin: Arc::new(Mutex::new(None)),
                cancel_tx,
                upstream_login_id: None,
            }),
        );
        self.journal.upsert(&view);
        tokio::spawn(monitor_process_login(
            child,
            public,
            cancel_rx,
            self.journal.clone(),
        ));
        Ok(view)
    }

    async fn register_attempt(
        &self,
        view: ProviderLoginAttempt,
        upstream_login_id: Option<String>,
        child: Child,
        stdin: Arc<Mutex<Option<ChildStdin>>>,
        reader: BufReader<ChildStdout>,
        kind: MonitorKind,
    ) -> Result<ProviderLoginAttempt> {
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let public = Arc::new(RwLock::new(view.clone()));
        let control = Arc::new(AttemptControl {
            view: public.clone(),
            stdin: stdin.clone(),
            cancel_tx,
            upstream_login_id: upstream_login_id.clone(),
        });
        self.attempts
            .write()
            .await
            .insert(view.login_id.clone(), control);
        self.journal.upsert(&view);
        tokio::spawn(monitor_login(
            kind,
            child,
            stdin,
            reader,
            public,
            cancel_rx,
            upstream_login_id,
            self.journal.clone(),
        ));
        Ok(view)
    }

    async fn active_attempt(&self, provider_id: &str) -> Option<ProviderLoginAttempt> {
        self.latest(provider_id)
            .await
            .filter(|view| view.is_active())
    }

    async fn idempotent_attempt(
        &self,
        provider_id: &str,
        request_id: Option<&str>,
    ) -> Option<ProviderLoginAttempt> {
        let request_id = request_id?;
        let controls = self
            .attempts
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for control in controls {
            let view = control.view.read().await;
            if view.provider_id == provider_id && view.request_id.as_deref() == Some(request_id) {
                return Some(view.clone());
            }
        }
        None
    }

    async fn cancel_active(&self, provider_id: &str) {
        if let Some(active) = self.active_attempt(provider_id).await {
            let _ = self.cancel(provider_id, &active.login_id).await;
        }
    }

    async fn prune(&self) {
        let cutoff = now_ms().saturating_sub(ATTEMPT_RETENTION_MS);
        let controls = self
            .attempts
            .read()
            .await
            .iter()
            .map(|(id, control)| (id.clone(), control.clone()))
            .collect::<Vec<_>>();
        let mut expired = Vec::new();
        for (id, control) in controls {
            let view = control.view.read().await;
            if view.is_terminal() && view.updated_at_ms < cutoff {
                expired.push(id);
            }
        }
        if !expired.is_empty() {
            self.attempts
                .write()
                .await
                .retain(|id, _| !expired.contains(id));
            self.journal.remove(&expired);
        }
    }
}

#[derive(Clone, Copy)]
enum MonitorKind {
    Codex,
    Gemini,
}

async fn monitor_process_login(
    mut child: Child,
    view: Arc<RwLock<ProviderLoginAttempt>>,
    mut cancel_rx: watch::Receiver<bool>,
    journal: ProviderAuthAttemptStore,
) {
    tokio::select! {
        _ = cancel_rx.changed() => {
            set_attempt_state(&view, "canceled", None, None, &journal).await;
            stop_child(&mut child).await;
        }
        _ = tokio::time::sleep(LOGIN_TIMEOUT) => {
            set_attempt_state(
                &view,
                "expired",
                Some("登录等待已超过 15 分钟，请重新发起。".to_string()),
                Some("login_expired"),
                &journal,
            ).await;
            stop_child(&mut child).await;
        }
        result = child.wait() => {
            match result {
                Ok(status) if status.success() => set_attempt_state(&view, "completed", None, None, &journal).await,
                Ok(status) => set_attempt_state(
                    &view,
                    "failed",
                    Some(format!("官方 CLI 登录进程退出码 {:?}。", status.code())),
                    Some("provider_process_failed"),
                    &journal,
                ).await,
                Err(error) => set_attempt_state(&view, "failed", Some(safe_error(&error.to_string())), Some("provider_process_failed"), &journal).await,
            }
        }
    }
}

async fn monitor_login(
    kind: MonitorKind,
    mut child: Child,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    mut reader: BufReader<ChildStdout>,
    view: Arc<RwLock<ProviderLoginAttempt>>,
    mut cancel_rx: watch::Receiver<bool>,
    upstream_login_id: Option<String>,
    journal: ProviderAuthAttemptStore,
) {
    let deadline = Instant::now() + LOGIN_TIMEOUT;
    loop {
        let mut line = String::new();
        tokio::select! {
            _ = cancel_rx.changed() => {
                if matches!(kind, MonitorKind::Codex) {
                    if let Some(login_id) = upstream_login_id.as_deref() {
                        let _ = write_rpc(&stdin, &json!({
                            "method":"account/login/cancel","id":3,"params":{"loginId":login_id}
                        })).await;
                    }
                }
                set_attempt_state(&view, "canceled", None, None, &journal).await;
                break;
            }
            _ = tokio::time::sleep_until(deadline) => {
                set_attempt_state(&view, "expired", Some("登录等待已超过 15 分钟，请重新发起。".to_string()), Some("login_expired"), &journal).await;
                break;
            }
            read = reader.read_line(&mut line) => {
                match read {
                    Ok(0) => {
                        if view.read().await.is_active() {
                            set_attempt_state(&view, "failed", Some("官方 CLI 登录进程已提前退出。".to_string()), Some("provider_process_failed"), &journal).await;
                        }
                        break;
                    }
                    Ok(_) if line.len() > MAX_RPC_LINE_BYTES => {
                        set_attempt_state(&view, "failed", Some("官方 CLI 返回了超出限制的登录消息。".to_string()), Some("provider_protocol_error"), &journal).await;
                        break;
                    }
                    Ok(_) => {
                        if let Ok(message) = serde_json::from_str::<Value>(&line) {
                            if let Some((state, error)) = login_terminal_state(kind, &message, upstream_login_id.as_deref()) {
                                let code = (state == "failed").then_some("provider_login_failed");
                                set_attempt_state(&view, state, error, code, &journal).await;
                                break;
                            }
                        }
                    }
                    Err(error) => {
                        set_attempt_state(&view, "failed", Some(safe_error(&error.to_string())), Some("provider_protocol_error"), &journal).await;
                        break;
                    }
                }
            }
        }
    }
    stdin.lock().await.take();
    stop_child(&mut child).await;
}

fn login_terminal_state(
    kind: MonitorKind,
    message: &Value,
    upstream_login_id: Option<&str>,
) -> Option<(&'static str, Option<String>)> {
    match kind {
        MonitorKind::Codex => {
            if message.get("method")?.as_str()? != "account/login/completed" {
                return None;
            }
            let params = message.get("params")?;
            if let Some(expected) = upstream_login_id {
                if params.get("loginId").and_then(Value::as_str) != Some(expected) {
                    return None;
                }
            }
            if params.get("success").and_then(Value::as_bool) == Some(true) {
                Some(("completed", None))
            } else {
                Some((
                    "failed",
                    Some(safe_error(
                        params
                            .get("error")
                            .and_then(Value::as_str)
                            .unwrap_or("Codex 登录失败"),
                    )),
                ))
            }
        }
        MonitorKind::Gemini => {
            if message.get("id").and_then(Value::as_i64) != Some(2) {
                return None;
            }
            if let Some(error) = message.get("error") {
                Some(("failed", Some(rpc_error_message(error))))
            } else {
                Some(("completed", None))
            }
        }
    }
}

async fn set_attempt_state(
    view: &Arc<RwLock<ProviderLoginAttempt>>,
    state: &str,
    error: Option<String>,
    error_code: Option<&str>,
    journal: &ProviderAuthAttemptStore,
) {
    let mut view = view.write().await;
    view.state = state.to_string();
    view.error = error;
    view.error_code = error_code.map(ToOwned::to_owned);
    view.updated_at_ms = now_ms();
    journal.upsert(&view);
}

async fn spawn_rpc_process(
    program: &Path,
    args: &[&str],
) -> Result<(
    Child,
    Arc<Mutex<Option<ChildStdin>>>,
    BufReader<ChildStdout>,
)> {
    let mut std_command = elon_pc_dev_runtime::command_from_path(program);
    std_command.args(args);
    let mut command = Command::from(std_command);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .with_context(|| format!("无法启动官方 CLI：{}", program.display()))?;
    let stdin = child.stdin.take().context("官方 CLI 没有提供 stdin")?;
    let stdout = child.stdout.take().context("官方 CLI 没有提供 stdout")?;
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                line.clear();
            }
        });
    }
    Ok((
        child,
        Arc::new(Mutex::new(Some(stdin))),
        BufReader::new(stdout),
    ))
}

async fn write_rpc(stdin: &Arc<Mutex<Option<ChildStdin>>>, message: &Value) -> Result<()> {
    let mut guard = stdin.lock().await;
    let input = guard.as_mut().context("官方 CLI 登录连接已经关闭")?;
    let mut payload = serde_json::to_vec(message)?;
    payload.push(b'\n');
    input.write_all(&payload).await?;
    input.flush().await?;
    Ok(())
}

async fn read_response(
    reader: &mut BufReader<ChildStdout>,
    expected_id: i64,
    wait: Duration,
) -> Result<Value> {
    timeout(wait, async {
        loop {
            let mut line = String::new();
            let read = reader.read_line(&mut line).await?;
            if read == 0 {
                return Err(anyhow!("官方 CLI 在握手完成前退出"));
            }
            if line.len() > MAX_RPC_LINE_BYTES {
                return Err(anyhow!("官方 CLI 握手消息超出大小限制"));
            }
            let Ok(message) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if message.get("id").and_then(Value::as_i64) == Some(expected_id) {
                return Ok(message);
            }
        }
    })
    .await
    .map_err(|_| anyhow!("等待官方 CLI 登录握手超时"))?
}

fn rpc_result(message: Value) -> Result<Value> {
    if let Some(error) = message.get("error") {
        return Err(anyhow!(rpc_error_message(error)));
    }
    Ok(message.get("result").cloned().unwrap_or(Value::Null))
}

fn rpc_error_message(error: &Value) -> String {
    safe_error(
        error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("官方 CLI 返回登录错误"),
    )
}

fn safe_error(message: &str) -> String {
    crate::node_agent_cli_redaction::redact_text(message)
        .chars()
        .take(500)
        .collect()
}

async fn stop_child(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill().await;
    }
    let _ = child.wait().await;
}

async fn run_provider_command(program: &Path, args: &[&str]) -> Result<()> {
    let mut std_command = elon_pc_dev_runtime::command_from_path(program);
    std_command.args(args);
    let mut command = Command::from(std_command);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .with_context(|| format!("无法启动官方 CLI：{}", program.display()))?;
    let status = timeout(Duration::from_secs(30), child.wait())
        .await
        .map_err(|_| anyhow!("等待官方 CLI 退出账号超时"))??;
    if !status.success() {
        return Err(anyhow!("官方 CLI 退出账号失败，退出码 {:?}", status.code()));
    }
    Ok(())
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
