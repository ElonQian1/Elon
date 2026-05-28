use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    sync::mpsc::UnboundedSender,
};

use crate::types::WsMessage;

pub(crate) async fn send_cli_heartbeat(
    tx: UnboundedSender<String>,
    last_activity_ms: Arc<AtomicU64>,
) {
    // CLI 静默超过这个阈值才发心跳；CLI 还在出 stdout 时不干扰。
    const SILENCE_THRESHOLD: Duration = Duration::from_secs(20);
    // 检查频率：足够频繁到能及时发出心跳，但不过于浪费调度。
    const TICK_INTERVAL: Duration = Duration::from_secs(5);
    let started_at = Instant::now();
    let mut last_heartbeat: Option<Instant> = None;
    loop {
        tokio::time::sleep(TICK_INTERVAL).await;
        let now_ms = current_unix_millis();
        let last_ms = last_activity_ms.load(Ordering::Relaxed);
        let silence = Duration::from_millis(now_ms.saturating_sub(last_ms));
        if silence < SILENCE_THRESHOLD {
            continue;
        }
        // 静默期间最多每 15s 重发一次，避免刷屏。
        if let Some(prev) = last_heartbeat {
            if prev.elapsed() < Duration::from_secs(15) {
                continue;
            }
        }
        let elapsed_secs = started_at.elapsed().as_secs();
        let silence_secs = silence.as_secs();
        let message = if silence_secs < 60 {
            format!("AI 还在思考（已等待 {} 秒）…", elapsed_secs)
        } else {
            format!(
                "AI 还在后台处理（已等待 {} 秒，本轮已静默 {} 秒）…",
                elapsed_secs, silence_secs
            )
        };
        if tx.send(WsMessage::progress(message).to_json()).is_err() {
            break;
        }
        last_heartbeat = Some(Instant::now());
    }
}

pub(crate) fn current_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub(crate) async fn read_cli_stream<R>(
    reader: R,
    last_activity_ms: Option<Arc<AtomicU64>>,
    progress_tx: Option<UnboundedSender<String>>,
) -> String
where
    R: AsyncRead + Unpin,
{
    let mut lines = BufReader::new(reader).lines();
    let mut collected = String::new();

    while let Ok(Some(line)) = lines.next_line().await {
        if let Some(ts) = last_activity_ms.as_ref() {
            ts.store(current_unix_millis(), Ordering::Relaxed);
        }
        if let Some(tx) = progress_tx.as_ref() {
            for message in crate::codex_stream::stream_event_to_ws_messages(&line) {
                let _ = tx.send(message);
            }
        }
        collected.push_str(&line);
        collected.push('\n');
    }

    collected
}
