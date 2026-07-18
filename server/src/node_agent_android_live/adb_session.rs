use std::time::Duration;

use anyhow::{anyhow, bail, Result};

use crate::node_agent_android_inspector::adb_capture::wake_device_for_user_interaction;
use crate::node_agent_android_inspector::adb_command::{
    run_adb_text, validate_device_id, validate_package_name,
};

use super::broker::LiveUiSession;

pub(crate) const DEFAULT_DEVICE_PORT: u16 = 38_917;
const RUNTIME_RECEIVER: &str = "com.elon.uiruntime.view.UiRuntimeControlReceiver";
const START_ACTION: &str = "com.elon.uiruntime.START";
const STOP_ACTION: &str = "com.elon.uiruntime.STOP";
const MAX_DIAGNOSTIC_OUTPUT: usize = 128 * 1024;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeStartEvidence {
    device_port: u16,
    host_port: u16,
    app_launch: String,
    receiver_broadcast: String,
    reverse_list: String,
}

struct RuntimeFailureSummary<'a> {
    app_launch: &'a str,
    process: &'a str,
    receiver_broadcast: &'a str,
    device_port: u16,
    host_port: u16,
    reverse: &'a str,
    stage: &'a str,
    connected: bool,
    runtime_build_id: &'a str,
    node_count: usize,
    last_error: &'a str,
    logs: &'a str,
}

pub(crate) async fn start_runtime(
    session: &LiveUiSession,
    host_port: u16,
) -> Result<RuntimeStartEvidence> {
    validate_device_id(&session.device_id)?;
    validate_package_name(&session.package_name)?;
    let device_port = session.device_port;
    session.record_runtime_stage("REVERSE_CONFIGURING").await;
    run_adb_text(
        &[
            "-s".to_string(),
            session.device_id.clone(),
            "reverse".to_string(),
            format!("tcp:{device_port}"),
            format!("tcp:{host_port}"),
        ],
        Duration::from_secs(8),
        64 * 1024,
    )
    .await
    .map_err(|error| sanitized_adb_error("配置 adb reverse 失败", error, &session.token))?;
    let reverse_list = read_reverse_list(session)
        .await
        .map_err(|error| sanitized_adb_error("读取 adb reverse 状态失败", error, &session.token))?;
    if !reverse_mapping_matches(&reverse_list, device_port, host_port) {
        bail!(
            "adb reverse 未建立预期映射：设备 tcp:{device_port} -> 主机 tcp:{host_port}；实际={}",
            compact_diagnostic(&sanitize_diagnostic_text(&reverse_list, &session.token))
        );
    }
    session.record_runtime_stage("REVERSE_VERIFIED").await;
    // Some OEM systems (notably MIUI) defer explicit broadcasts to cached
    // background processes. A LIVE session is user-initiated and needs the
    // target screen visible anyway, so resume the package task before sending
    // the runtime control broadcast. `monkey` resolves the launcher activity
    // without requiring the PC editor to know the app's concrete Activity.
    wake_device_for_user_interaction(&session.device_id).await;
    let app_launch = run_adb_text(
        &[
            "-s".to_string(),
            session.device_id.clone(),
            "shell".to_string(),
            "monkey".to_string(),
            "-p".to_string(),
            session.package_name.clone(),
            "-c".to_string(),
            "android.intent.category.LAUNCHER".to_string(),
            "1".to_string(),
        ],
        Duration::from_secs(10),
        128 * 1024,
    )
    .await
    .map_err(|error| sanitized_adb_error("启动 Debug 应用失败", error, &session.token))?;
    session.record_runtime_stage("APP_LAUNCHED").await;
    tokio::time::sleep(Duration::from_millis(650)).await;
    let component = format!("{}/{}", session.package_name, RUNTIME_RECEIVER);
    session
        .record_runtime_stage("RECEIVER_BROADCAST_SENT")
        .await;
    let receiver_broadcast = run_adb_text(
        &[
            "-s".to_string(),
            session.device_id.clone(),
            "shell".to_string(),
            "am".to_string(),
            "broadcast".to_string(),
            "-a".to_string(),
            START_ACTION.to_string(),
            "-n".to_string(),
            component,
            "--es".to_string(),
            "session_id".to_string(),
            session.id.clone(),
            "--es".to_string(),
            "session_token".to_string(),
            session.token.clone(),
            "--ei".to_string(),
            "device_port".to_string(),
            device_port.to_string(),
        ],
        Duration::from_secs(10),
        128 * 1024,
    )
    .await
    .map_err(|error| {
        sanitized_adb_error(
            "发送 Debug Runtime Receiver 广播失败",
            error,
            &session.token,
        )
    })?;
    let safe_receiver = sanitize_diagnostic_text(&receiver_broadcast, &session.token);
    if safe_receiver.contains("result=-1") || safe_receiver.contains("Error:") {
        bail!("启动 Android Live Runtime 失败: {}", safe_receiver.trim());
    }
    Ok(RuntimeStartEvidence {
        device_port,
        host_port,
        app_launch: sanitize_diagnostic_text(&app_launch, &session.token),
        receiver_broadcast: safe_receiver,
        reverse_list: sanitize_diagnostic_text(&reverse_list, &session.token),
    })
}

pub(crate) async fn runtime_failure_diagnostics(
    session: &LiveUiSession,
    start: Option<&RuntimeStartEvidence>,
) -> String {
    let process = run_adb_text(
        &[
            "-s".to_string(),
            session.device_id.clone(),
            "shell".to_string(),
            "pidof".to_string(),
            session.package_name.clone(),
        ],
        Duration::from_secs(4),
        16 * 1024,
    )
    .await
    .map(|output| {
        if output.trim().is_empty() {
            "未运行".to_string()
        } else {
            format!("运行中(pid={})", compact_diagnostic(&output))
        }
    })
    .unwrap_or_else(|error| format!("查询失败({})", compact_diagnostic(&error.to_string())));
    let reverse_list = read_reverse_list(session)
        .await
        .map(|output| sanitize_diagnostic_text(&output, &session.token))
        .unwrap_or_else(|error| format!("读取失败({})", error));
    let logs = read_runtime_logs(session)
        .await
        .map(|output| runtime_log_summary(&sanitize_diagnostic_text(&output, &session.token)))
        .unwrap_or_else(|error| format!("读取失败({})", compact_diagnostic(&error.to_string())));
    let view = session.view().await;
    let stage = session
        .runtime_stage()
        .await
        .unwrap_or_else(|| "NOT_STARTED".to_string());
    let (device_port, host_port, app_launch, receiver_broadcast) = match start {
        Some(start) => (
            start.device_port,
            start.host_port,
            compact_diagnostic(&start.app_launch),
            compact_diagnostic(&start.receiver_broadcast),
        ),
        None => (
            session.device_port,
            crate::node_agent_admin_open::admin_port_from_env(),
            "未记录".to_string(),
            "未记录".to_string(),
        ),
    };
    let reverse = match start {
        Some(start) => format!(
            "当前={}；启动时={}",
            reverse_mapping_summary(&reverse_list, device_port, host_port),
            reverse_mapping_summary(&start.reverse_list, device_port, host_port),
        ),
        None => reverse_mapping_summary(&reverse_list, device_port, host_port),
    };
    let last_error = view
        .last_error
        .as_deref()
        .map(|value| compact_diagnostic(&sanitize_diagnostic_text(value, &session.token)))
        .unwrap_or_else(|| "无".to_string());
    format_runtime_failure_summary(RuntimeFailureSummary {
        app_launch: &app_launch,
        process: &process,
        receiver_broadcast: &receiver_broadcast,
        device_port,
        host_port,
        reverse: &reverse,
        stage: &stage,
        connected: view.connected,
        runtime_build_id: view.runtime_build_id.as_deref().unwrap_or("无"),
        node_count: view.node_count,
        last_error: &last_error,
        logs: &logs,
    })
}

fn format_runtime_failure_summary(summary: RuntimeFailureSummary<'_>) -> String {
    format!(
        "应用启动={}；应用进程={}；Receiver={}；Runtime端口=设备 tcp:{} -> 主机 tcp:{}；reverse={}；WebSocket握手阶段={}；connected={}；runtimeBuildId={}；nodeCount={}；lastError={}；YilongUiRuntime日志={}",
        summary.app_launch,
        summary.process,
        summary.receiver_broadcast,
        summary.device_port,
        summary.host_port,
        summary.reverse,
        summary.stage,
        summary.connected,
        summary.runtime_build_id,
        summary.node_count,
        summary.last_error,
        summary.logs,
    )
}

async fn read_reverse_list(session: &LiveUiSession) -> Result<String> {
    run_adb_text(
        &[
            "-s".to_string(),
            session.device_id.clone(),
            "reverse".to_string(),
            "--list".to_string(),
        ],
        Duration::from_secs(5),
        64 * 1024,
    )
    .await
}

async fn read_runtime_logs(session: &LiveUiSession) -> Result<String> {
    run_adb_text(
        &[
            "-s".to_string(),
            session.device_id.clone(),
            "logcat".to_string(),
            "-d".to_string(),
            "-t".to_string(),
            "160".to_string(),
            "YilongUiRuntime:I".to_string(),
            "*:S".to_string(),
        ],
        Duration::from_secs(6),
        MAX_DIAGNOSTIC_OUTPUT,
    )
    .await
}

fn reverse_mapping_matches(output: &str, device_port: u16, host_port: u16) -> bool {
    let device = format!("tcp:{device_port}");
    let host = format!("tcp:{host_port}");
    output
        .lines()
        .any(|line| line.contains(&device) && line.contains(&host))
}

fn reverse_mapping_summary(output: &str, device_port: u16, host_port: u16) -> String {
    let status = if reverse_mapping_matches(output, device_port, host_port) {
        "已验证"
    } else {
        "缺失或端口不匹配"
    };
    format!("{status}({})", compact_diagnostic(output))
}

fn runtime_log_summary(output: &str) -> String {
    let mut lines = output
        .lines()
        .map(compact_diagnostic)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if lines.len() > 12 {
        lines.drain(..lines.len() - 12);
    }
    if lines.is_empty() {
        "无相关日志".to_string()
    } else {
        lines.join(" | ")
    }
}

fn compact_diagnostic(value: &str) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = compact.chars();
    let visible = chars.by_ref().take(360).collect::<String>();
    if chars.next().is_some() {
        format!("{visible}...")
    } else {
        visible
    }
}

fn sanitize_diagnostic_text(value: &str, session_token: &str) -> String {
    let without_literal = if session_token.is_empty() {
        value.to_string()
    } else {
        value.replace(session_token, "<redacted>")
    };
    ["token=", "session_token=", "session_token "]
        .into_iter()
        .fold(without_literal, redact_marker_value)
}

fn redact_marker_value(value: String, marker: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find(marker) {
        let marker_start = cursor + relative;
        let value_start = marker_start + marker.len();
        output.push_str(&value[cursor..value_start]);
        output.push_str("<redacted>");
        let mut end = value_start;
        for (offset, ch) in value[value_start..].char_indices() {
            if ch.is_whitespace() || matches!(ch, '&' | ',' | '"' | '\'' | '}') {
                break;
            }
            end = value_start + offset + ch.len_utf8();
        }
        cursor = end;
    }
    output.push_str(&value[cursor..]);
    output
}

fn sanitized_adb_error(label: &str, error: anyhow::Error, session_token: &str) -> anyhow::Error {
    anyhow!(
        "{label}: {}",
        sanitize_diagnostic_text(&error.to_string(), session_token)
    )
}

pub(crate) async fn stop_runtime(session: &LiveUiSession) -> Result<()> {
    let component = format!("{}/{}", session.package_name, RUNTIME_RECEIVER);
    let _ = run_adb_text(
        &[
            "-s".to_string(),
            session.device_id.clone(),
            "shell".to_string(),
            "am".to_string(),
            "broadcast".to_string(),
            "-a".to_string(),
            STOP_ACTION.to_string(),
            "-n".to_string(),
            component,
            "--es".to_string(),
            "session_id".to_string(),
            session.id.clone(),
        ],
        Duration::from_secs(8),
        64 * 1024,
    )
    .await;
    // The reverse rule is shared by consecutive browser sessions on the same
    // device port. Removing it while an older React effect is winding down can
    // disconnect the newer Runtime. A later START idempotently replaces it.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reverse_diagnostics_require_both_device_and_host_ports() {
        let output = "UsbFfs tcp:38917 tcp:7800\n";
        assert!(reverse_mapping_matches(output, 38_917, 7_800));
        assert!(!reverse_mapping_matches(output, 38_917, 7_799));
    }

    #[test]
    fn runtime_diagnostics_redact_session_tokens_and_query_values() {
        let secret = "0123456789abcdef0123456789abcdef";
        let raw = format!(
            "ws://127.0.0.1:38917/runtime?sessionId=live_1&token={secret} session_token {secret}"
        );
        let safe = sanitize_diagnostic_text(&raw, secret);
        assert!(!safe.contains(secret));
        assert!(safe.contains("token=<redacted>"));
        assert!(safe.contains("session_token <redacted>"));
    }

    #[test]
    fn startup_and_handshake_failure_summary_is_actionable() {
        let summary = format_runtime_failure_summary(RuntimeFailureSummary {
            app_launch: "Events injected: 1",
            process: "运行中(pid=42)",
            receiver_broadcast: "Broadcast completed: result=0",
            device_port: 38_917,
            host_port: 7_800,
            reverse: "缺失或端口不匹配",
            stage: "BROKER_WELCOME_SENT",
            connected: true,
            runtime_build_id: "无",
            node_count: 0,
            last_error: "Runtime 协议版本不兼容",
            logs: "YilongUiRuntime: WebSocket failed",
        });
        for expected in [
            "应用启动=Events injected: 1",
            "Receiver=Broadcast completed: result=0",
            "设备 tcp:38917 -> 主机 tcp:7800",
            "reverse=缺失或端口不匹配",
            "WebSocket握手阶段=BROKER_WELCOME_SENT",
            "runtimeBuildId=无",
            "YilongUiRuntime日志=YilongUiRuntime: WebSocket failed",
        ] {
            assert!(summary.contains(expected), "missing {expected}: {summary}");
        }
    }

    #[test]
    fn runtime_log_summary_is_bounded_to_recent_lines() {
        let logs = (0..20)
            .map(|index| format!("YilongUiRuntime line {index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let summary = runtime_log_summary(&logs);
        assert!(!summary.contains("line 0 |"));
        assert!(summary.contains("line 19"));
        assert!(summary.len() < 4_500);
    }
}
