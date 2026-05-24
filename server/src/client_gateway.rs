use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::{header, StatusCode},
    response::IntoResponse,
};
use base64::{engine::general_purpose, Engine as _};
use std::{
    collections::{HashMap, HashSet},
    path::Path as FsPath,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, LazyLock,
    },
    time::Duration,
};
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::info;

/// 连接级"已订阅的 job 锁键"集合，避免同一 WebSocket 上同一会话被多个 forwarder 重复订阅
/// 同一份 broadcast 事件造成客户端收到重复消息。
type ConnForwarderSet = Arc<Mutex<HashSet<String>>>;

use crate::{agent, client_protocol, tools, types};
use client_protocol::AgentRequest;
use types::AppState;

static LEGACY_WS_JOBS: LazyLock<Mutex<HashMap<String, Arc<LegacyWsJob>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
const MAX_ATTACHMENTS_PER_MESSAGE: usize = 6;
const MAX_ATTACHMENT_BYTES: usize = 12 * 1024 * 1024;

struct LegacyWsJob {
    key: String,
    fingerprint: String,
    backlog: Mutex<Vec<String>>,
    broadcaster: broadcast::Sender<String>,
    finished: AtomicBool,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(socket: WebSocket, state: Arc<AppState>) {
    use futures::{SinkExt, StreamExt};
    let (mut sender, mut receiver) = socket.split();

    info!("new WebSocket connection");

    // 出站消息统一从这个 mpsc 队列经一个 sender task 写到 socket，避免多个 forwarder
    // 同时 await sender.send() 导致顺序错乱或 borrow 冲突。
    let (out_tx, mut out_rx) = mpsc::channel::<Message>(256);
    let forwarders: ConnForwarderSet = Arc::new(Mutex::new(HashSet::new()));

    let sender_task = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
        let _ = sender.close().await;
    });

    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(error) => {
                info!("WebSocket receive error: {}", error);
                break;
            }
        };
        match msg {
            Message::Ping(payload) => {
                if out_tx.send(Message::Pong(payload)).await.is_err() {
                    break;
                }
            }
            Message::Pong(_) => continue,
            Message::Close(_) => break,
            Message::Binary(_) => continue,
            Message::Text(text) => {
                info!("received WebSocket message: {} bytes", text.len());
                let state_clone = state.clone();
                let tx_clone = out_tx.clone();
                let forwarders_clone = forwarders.clone();
                // 关键改动：每条 Text 消息立即返回，开协程处理；reader loop 不阻塞，
                // 后续消息可以马上进入处理流程，不会被前一条 job 卡住。
                tokio::spawn(async move {
                    process_incoming_text(text, state_clone, tx_clone, forwarders_clone).await;
                });
            }
        }
    }

    drop(out_tx);
    let _ = sender_task.await;
    info!("WebSocket connection closed");
}

async fn process_incoming_text(
    text: String,
    state: Arc<AppState>,
    out_tx: mpsc::Sender<Message>,
    forwarders: ConnForwarderSet,
) {
    let request =
        materialize_attachments(client_protocol::parse_client_message(&text), &state).await;
    info!(
        "dispatching request: user_id={} workspace_user_id={} conversation_id={} agent={:?} chars={}",
        request.user_id,
        request.workspace_user_id,
        request.conversation_id,
        request.agent,
        request.content.chars().count()
    );
    if let Some(response) = quick_apk_delivery_response(&request, &state).await {
        let _ = out_tx.send(Message::Text(response)).await;
        return;
    }
    let job = get_or_start_legacy_job(request, state.clone()).await;
    let key = job.key.clone();

    // 连接维度去重：同一会话已经有 forwarder 在订阅 broadcast，则新消息只需触发 job 内的
    // "已加入队列"提示（get_or_start_legacy_job 已发出），无需再次订阅，避免重复推送。
    {
        let mut set = forwarders.lock().await;
        if !set.insert(key.clone()) {
            return;
        }
    }

    let mut job_rx = job.broadcaster.subscribe();
    let backlog = job.backlog.lock().await.clone();
    let mut replayed_terminal = false;
    for progress in backlog {
        if out_tx.send(Message::Text(progress.clone())).await.is_err() {
            forwarders.lock().await.remove(&key);
            return;
        }
        if is_terminal_ws_message(&progress) {
            replayed_terminal = true;
            break;
        }
    }
    if replayed_terminal {
        forwarders.lock().await.remove(&key);
        return;
    }

    loop {
        match job_rx.recv().await {
            Ok(progress) => {
                if out_tx.send(Message::Text(progress)).await.is_err() {
                    break;
                }
                if job.finished.load(Ordering::SeqCst) {
                    break;
                }
            }
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
    forwarders.lock().await.remove(&key);
}

async fn materialize_attachments(mut request: AgentRequest, state: &AppState) -> AgentRequest {
    if request.attachments.is_empty() {
        return request;
    }

    let workspace = state.get_user_workspace(&request.workspace_user_id);
    let attachments_dir = workspace.join("attachments");
    if let Err(error) = tokio::fs::create_dir_all(&attachments_dir).await {
        request.content = format!(
            "{}\n\n附件处理失败：无法创建附件目录 {}：{}",
            request.content,
            attachments_dir.display(),
            error
        );
        request.attachments.clear();
        return request;
    }

    let mut notes = Vec::new();
    for (index, attachment) in request.attachments.iter().take(MAX_ATTACHMENTS_PER_MESSAGE).enumerate()
    {
        let decoded = match general_purpose::STANDARD.decode(attachment.data_base64.trim()) {
            Ok(bytes) if bytes.len() <= MAX_ATTACHMENT_BYTES => bytes,
            Ok(_) => {
                notes.push(format!(
                    "- {}：附件过大，已跳过。",
                    attachment.display_name
                ));
                continue;
            }
            Err(error) => {
                notes.push(format!(
                    "- {}：Base64 解析失败，已跳过：{}",
                    attachment.display_name, error
                ));
                continue;
            }
        };
        let file_name = unique_attachment_name(&attachments_dir, index, &attachment.file_name);
        let path = attachments_dir.join(&file_name);
        match tokio::fs::write(&path, decoded).await {
            Ok(()) => notes.push(format!(
                "- {} [{}; {}] -> {}",
                attachment.display_name,
                attachment.kind,
                attachment.mime_type,
                path.display()
            )),
            Err(error) => notes.push(format!(
                "- {}：写入失败，已跳过：{}",
                attachment.display_name, error
            )),
        }
    }

    if request.attachments.len() > MAX_ATTACHMENTS_PER_MESSAGE {
        notes.push(format!(
            "- 其余 {} 个附件超过本次上限，已忽略。",
            request.attachments.len() - MAX_ATTACHMENTS_PER_MESSAGE
        ));
    }

    if !notes.is_empty() {
        request.content = format!(
            "{}\n\n用户本次随消息上传了真实附件。附件已保存到当前工作目录，请直接读取这些路径，不要只按原文件名在项目根目录查找：\n{}",
            request.content,
            notes.join("\n")
        );
    }
    request.attachments.clear();
    request
}

fn unique_attachment_name(dir: &FsPath, index: usize, original: &str) -> String {
    let safe = safe_attachment_name(original);
    let mut candidate = format!("{:02}_{}", index + 1, safe);
    let mut suffix = 1;
    while dir.join(&candidate).exists() {
        candidate = format!("{:02}_{}_{}", index + 1, suffix, safe);
        suffix += 1;
    }
    candidate
}

fn safe_attachment_name(original: &str) -> String {
    let mut safe = original
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                Some(ch)
            } else if ch.is_whitespace() {
                Some('_')
            } else {
                None
            }
        })
        .collect::<String>();
    if safe.is_empty() || safe == "." || safe == ".." {
        safe = "attachment.bin".into();
    }
    if safe.len() > 80 {
        let ext = safe
            .rsplit_once('.')
            .map(|(_, ext)| format!(".{}", ext))
            .unwrap_or_default();
        safe = format!("attachment{}", ext);
    }
    safe
}

async fn get_or_start_legacy_job(request: AgentRequest, state: Arc<AppState>) -> Arc<LegacyWsJob> {
    // 锁键同时考虑 用户×项目×会话，避免同一用户同项目下不同会话互相阻塞。
    let key = legacy_job_key(&request.workspace_user_id, &request.conversation_id);
    let fingerprint = legacy_job_fingerprint(&request);

    {
        let mut jobs = LEGACY_WS_JOBS.lock().await;
        if let Some(existing) = jobs.get(&key) {
            if existing.fingerprint == fingerprint {
                return existing.clone();
            }
            if !existing.finished.load(Ordering::SeqCst) {
                let _ = existing.broadcaster.send(
                    types::WsMessage::Progress {
                        message: "上一轮任务仍在继续，正在保持后台处理。".into(),
                    }
                    .to_json(),
                );
                return existing.clone();
            }
            jobs.remove(&key);
        }

        let (broadcast_tx, _) = broadcast::channel::<String>(256);
        let job = Arc::new(LegacyWsJob {
            key: key.clone(),
            fingerprint: fingerprint.clone(),
            backlog: Mutex::new(Vec::new()),
            broadcaster: broadcast_tx,
            finished: AtomicBool::new(false),
        });
        jobs.insert(key.clone(), job.clone());

        let job_for_task = job.clone();
        tokio::spawn(async move {
            run_legacy_job(request, state, job_for_task).await;
        });

        job
    }
}

async fn run_legacy_job(request: AgentRequest, state: Arc<AppState>, job: Arc<LegacyWsJob>) {
    // 检查是否有被服务器重启中断的任务
    let was_interrupted = state
        .store
        .get_interrupted_ws_task(&request.workspace_user_id)
        .ok()
        .flatten()
        .is_some();

    // 标记当前任务为 running
    let _ = state
        .store
        .ws_task_started(&request.workspace_user_id, &request.content);

    // 如果上次任务被服务器重启中断，提前通知用户
    if was_interrupted {
        let warning = types::WsMessage::Progress {
            message: "⚠️ 上次任务被服务器重启中断，正在重新处理，请稍候...".into(),
        }
        .to_json();
        {
            let mut backlog = job.backlog.lock().await;
            backlog.push(warning.clone());
        }
        let _ = job.broadcaster.send(warning);
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let state_clone = state.clone();
    let request_for_agent = request.clone();

    let agent_task = tokio::spawn(async move {
        agent::run(
            &request_for_agent.user_id,
            &request_for_agent.workspace_user_id,
            &request_for_agent.content,
            request_for_agent.agent.as_deref(),
            &state_clone,
            tx,
        )
        .await;
    });

    let mut final_status = "done";
    while let Some(progress) = rx.recv().await {
        {
            let mut backlog = job.backlog.lock().await;
            backlog.push(progress.clone());
            if backlog.len() > 512 {
                let overflow = backlog.len() - 512;
                backlog.drain(0..overflow);
            }
        }
        let terminal = is_terminal_ws_message(&progress);
        if terminal && is_error_ws_message(&progress) {
            final_status = "error";
        }
        let _ = job.broadcaster.send(progress);
        if terminal {
            job.finished.store(true, Ordering::SeqCst);
            break;
        }
    }

    let _ = agent_task.await;
    job.finished.store(true, Ordering::SeqCst);

    // 任务完成，更新 DB 状态
    let _ = state
        .store
        .ws_task_finished(&request.workspace_user_id, final_status);

    let cleanup_key = job.key.clone();
    let cleanup_job = job.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30 * 60)).await;
        let mut jobs = LEGACY_WS_JOBS.lock().await;
        if jobs
            .get(&cleanup_key)
            .map(|existing| Arc::ptr_eq(existing, &cleanup_job))
            .unwrap_or(false)
        {
            jobs.remove(&cleanup_key);
        }
    });
}

fn legacy_job_key(workspace_user_id: &str, conversation_id: &str) -> String {
    format!("{}\u{1f}{}", workspace_user_id, conversation_id)
}

fn legacy_job_fingerprint(request: &AgentRequest) -> String {
    format!(
        "{}\u{1f}{}",
        request.agent.as_deref().unwrap_or(""),
        request.content
    )
}

fn is_terminal_ws_message(raw: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|value| {
            value
                .get("type")
                .and_then(|message_type| message_type.as_str())
                .map(|message_type| message_type == "done" || message_type == "error")
        })
        .unwrap_or(false)
}

fn is_error_ws_message(raw: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|v| v.get("type").and_then(|t| t.as_str()).map(|t| t == "error"))
        .unwrap_or(false)
}

async fn quick_apk_delivery_response(request: &AgentRequest, state: &AppState) -> Option<String> {
    if !looks_like_apk_delivery_request(&request.content) {
        return None;
    }

    let workspace = state.get_user_workspace(&request.workspace_user_id);
    let (message, apk_url) = if let Some(apk_path) = tools::find_latest_apk(&workspace) {
        let apk_name = apk_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if apk_name.is_empty() {
            (
                "当前没有找到可下载的 APK。需要先重新打包，完成后我再给你下载链接。".into(),
                None,
            )
        } else {
            (
                "APK 已生成，可以下载安装测试。".into(),
                Some(tools::stable_apk_url(&format!(
                    "{}/download/{}",
                    state.public_url.trim_end_matches('/'),
                    request.workspace_user_id
                ))),
            )
        }
    } else {
        (
            "当前没有找到可下载的 APK。需要先重新打包，完成后我再给你下载链接。".into(),
            None,
        )
    };
    Some(
        types::WsMessage::Done {
            message,
            apk_url,
            image_url: None,
        }
        .to_json(),
    )
}

fn looks_like_apk_delivery_request(message: &str) -> bool {
    let lower = message.to_lowercase();
    let asks_for_apk =
        lower.contains("apk") || lower.contains("安装包") || lower.contains("下载包");
    let asks_for_delivery = ["地址", "链接", "下载", "发给我", "给我", "做好", "做完", "完成"]
        .iter()
        .any(|word| lower.contains(word));
    asks_for_apk && asks_for_delivery
}

pub async fn download_apk(
    Path((user_id, filename)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err((StatusCode::BAD_REQUEST, "invalid filename".into()));
    }
    if !filename.ends_with(".apk") {
        return Err((
            StatusCode::BAD_REQUEST,
            "only APK downloads are allowed".into(),
        ));
    }

    let workspace = types::get_user_workspace(&state.workspace_root, &user_id);
    let apk_path = tools::find_download_apk(&workspace, &filename).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("APK file {} does not exist", filename),
        )
    })?;
    let download_name = apk_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&filename);

    let data = tokio::fs::read(&apk_path).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to read APK: {}", e),
        )
    })?;

    let response = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/vnd.android.package-archive",
        )
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", download_name),
        )
        .body(Body::from(data))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(response)
}
