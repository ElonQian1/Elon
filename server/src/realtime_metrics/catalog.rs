use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RealtimeDiagnosticChannel {
    pub id: &'static str,
    pub business_boundary: &'static str,
    pub entry_modules: &'static [&'static str],
    pub close_reason_source: &'static str,
    pub metric_variant: &'static str,
    pub sync_targets: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RealtimeDiagnosticCloseReason {
    pub id: &'static str,
    pub source: &'static str,
    pub category: &'static str,
    pub alert_bucket: Option<&'static str>,
    pub meaning: &'static str,
    pub first_check: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RealtimeDiagnosticsCatalog {
    pub version: &'static str,
    pub channels: &'static [RealtimeDiagnosticChannel],
    pub close_reasons: &'static [RealtimeDiagnosticCloseReason],
    pub change_rules: &'static [&'static str],
}

const REALTIME_DIAGNOSTIC_CHANNELS: &[RealtimeDiagnosticChannel] = &[
    RealtimeDiagnosticChannel {
        id: "app_notify",
        business_boundary: "Legacy mobile notify and upgrade prompt websocket",
        entry_modules: &["server/src/app_update.rs"],
        close_reason_source: "WsCloseReason",
        metric_variant: "RealtimeChannel::AppNotify",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/check-realtime-ownership.ps1",
        ],
    },
    RealtimeDiagnosticChannel {
        id: "global_app",
        business_boundary: "Global app websocket for online presence and common pushes",
        entry_modules: &["server/src/global_ws.rs"],
        close_reason_source: "WsCloseReason",
        metric_variant: "RealtimeChannel::GlobalApp",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/check-realtime-ownership.ps1",
        ],
    },
    RealtimeDiagnosticChannel {
        id: "project_ws",
        business_boundary: "Project task websocket for execution, approvals, and task status",
        entry_modules: &["server/src/project_ws_session.rs"],
        close_reason_source: "WsCloseReason",
        metric_variant: "RealtimeChannel::ProjectWs",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/test-admin-realtime-health.js",
            "scripts/check-realtime-ownership.ps1",
        ],
    },
    RealtimeDiagnosticChannel {
        id: "voice_transcribe",
        business_boundary: "Realtime speech-to-text websocket",
        entry_modules: &[
            "server/src/voice_ws_transcribe.rs",
            "server/src/voice_ws_transcribe_impl.rs",
        ],
        close_reason_source: "WsCloseReason",
        metric_variant: "RealtimeChannel::VoiceTranscribe",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/test-admin-realtime-health.js",
            "scripts/check-realtime-ownership.ps1",
        ],
    },
    RealtimeDiagnosticChannel {
        id: "voice_realtime_chat",
        business_boundary: "Realtime AI voice chat websocket",
        entry_modules: &[
            "server/src/voice_ws_realtime_chat.rs",
            "server/src/voice_ws_realtime_chat_impl.rs",
        ],
        close_reason_source: "WsCloseReason",
        metric_variant: "RealtimeChannel::VoiceRealtimeChat",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/test-admin-realtime-health.js",
            "scripts/check-realtime-ownership.ps1",
        ],
    },
    RealtimeDiagnosticChannel {
        id: "voice_virtual_mic",
        business_boundary: "Virtual microphone realtime input websocket",
        entry_modules: &["server/src/voice_ws_virtual_mic.rs"],
        close_reason_source: "WsCloseReason",
        metric_variant: "RealtimeChannel::VoiceVirtualMic",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/test-admin-realtime-health.js",
            "scripts/check-realtime-ownership.ps1",
        ],
    },
    RealtimeDiagnosticChannel {
        id: "homecli_agent",
        business_boundary: "HomeCLI/PC agent reverse websocket for PC CLI dispatch",
        entry_modules: &["server/src/homecli_agent/agent_session.rs"],
        close_reason_source: "AgentSessionCloseReason",
        metric_variant: "RealtimeChannel::HomecliAgent",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/check-realtime-ownership.ps1",
            "server/src/homecli_agent_tests.rs",
        ],
    },
    RealtimeDiagnosticChannel {
        id: "peer_relay",
        business_boundary: "Mobile P2P APK relay and seeder transfer websocket",
        entry_modules: &["server/src/peer_relay.rs"],
        close_reason_source: "PeerWsCloseReason",
        metric_variant: "RealtimeChannel::PeerRelay",
        sync_targets: &[
            "docs/realtime-operations-runbook.md",
            "docs/realtime-channel-ownership.md",
            "scripts/check-realtime-ownership.ps1",
        ],
    },
];

const REALTIME_DIAGNOSTIC_CLOSE_REASONS: &[RealtimeDiagnosticCloseReason] = &[
    RealtimeDiagnosticCloseReason {
        id: "peer_closed",
        source: "WsCloseReason|PeerWsCloseReason",
        category: "normal_close",
        alert_bucket: None,
        meaning: "Peer sent a websocket close frame or closed intentionally.",
        first_check: "Check client navigation, backgrounding, refresh, or intentional stop flow.",
    },
    RealtimeDiagnosticCloseReason {
        id: "client_control_close",
        source: "WsCloseReason",
        category: "normal_close",
        alert_bucket: None,
        meaning: "Client sent an application-level control message that closes the realtime flow.",
        first_check: "Check voice/task stop order and client control message handling.",
    },
    RealtimeDiagnosticCloseReason {
        id: "reader_ended",
        source: "WsCloseReason",
        category: "ended",
        alert_bucket: None,
        meaning: "Websocket read stream ended without a specific read error.",
        first_check: "Check proxy idle timeout, client process exit, and network transitions.",
    },
    RealtimeDiagnosticCloseReason {
        id: "peer_reader_ended",
        source: "PeerWsCloseReason",
        category: "ended",
        alert_bucket: None,
        meaning: "Peer relay seeder read stream ended during registration or transfer.",
        first_check: "Check seeder app lifecycle, local network stability, and relay registry cleanup.",
    },
    RealtimeDiagnosticCloseReason {
        id: "reader_closed",
        source: "AgentSessionCloseReason",
        category: "ended",
        alert_bucket: None,
        meaning: "HomeCLI agent reader closed normally.",
        first_check: "Check PC agent restart, server upgrade, or intentional reconnect.",
    },
    RealtimeDiagnosticCloseReason {
        id: "reader_shutdown",
        source: "AgentSessionCloseReason",
        category: "shutdown",
        alert_bucket: None,
        meaning: "HomeCLI agent session reader was shut down by server-side lifecycle control.",
        first_check: "Check server restart, deployment, or writer-side failure that triggered shutdown.",
    },
    RealtimeDiagnosticCloseReason {
        id: "read_error",
        source: "WsCloseReason",
        category: "read_error",
        alert_bucket: Some("read_error"),
        meaning: "Server failed to read a websocket frame.",
        first_check: "Check client network quality, websocket protocol compatibility, and reverse proxy behavior.",
    },
    RealtimeDiagnosticCloseReason {
        id: "reader_error",
        source: "AgentSessionCloseReason",
        category: "read_error",
        alert_bucket: Some("read_error"),
        meaning: "Server failed to read from the HomeCLI agent websocket.",
        first_check: "Check PC agent connectivity, local runtime restart, and node false-online state.",
    },
    RealtimeDiagnosticCloseReason {
        id: "peer_read_error",
        source: "PeerWsCloseReason",
        category: "read_error",
        alert_bucket: Some("read_error"),
        meaning: "Server failed to read from the peer relay seeder websocket.",
        first_check: "Check seeder network, app lifecycle, and APK transfer interruption.",
    },
    RealtimeDiagnosticCloseReason {
        id: "write_failed",
        source: "WsCloseReason",
        category: "write_failure",
        alert_bucket: Some("write_failure"),
        meaning: "Server failed to write to a websocket peer.",
        first_check: "Check half-open clients, slow consumers, and proxy disconnects.",
    },
    RealtimeDiagnosticCloseReason {
        id: "pong_write_failed",
        source: "WsCloseReason",
        category: "write_failure",
        alert_bucket: Some("write_failure"),
        meaning: "Server failed to write Pong after receiving Ping.",
        first_check: "Check half-open connection, network jitter, and proxy keepalive behavior.",
    },
    RealtimeDiagnosticCloseReason {
        id: "writer_closed",
        source: "AgentSessionCloseReason",
        category: "write_failure",
        alert_bucket: Some("write_failure"),
        meaning: "HomeCLI agent writer side closed while the session was active.",
        first_check: "Check PC agent writer loop, outbound queue closure, and pending CLI failure messages.",
    },
    RealtimeDiagnosticCloseReason {
        id: "peer_write_error",
        source: "PeerWsCloseReason",
        category: "write_failure",
        alert_bucket: Some("write_failure"),
        meaning: "Server failed to write to a peer relay seeder websocket.",
        first_check: "Check relay command delivery, seeder disconnects, and APK transfer backpressure.",
    },
    RealtimeDiagnosticCloseReason {
        id: "reader_timeout",
        source: "AgentSessionCloseReason",
        category: "timeout",
        alert_bucket: Some("timeout"),
        meaning: "HomeCLI agent reader timed out.",
        first_check: "Check PC sleep, node false-online state, heartbeat loss, and local network/proxy issues.",
    },
    RealtimeDiagnosticCloseReason {
        id: "request_channel_closed",
        source: "PeerWsCloseReason",
        category: "relay_control",
        alert_bucket: None,
        meaning: "Peer relay request channel closed before the transfer completed.",
        first_check: "Check relay registry cleanup, downloader cancellation, and seeder request lifecycle.",
    },
];

const REALTIME_DIAGNOSTIC_CHANGE_RULES: &[&str] = &[
    "Add every new RealtimeChannel label to the diagnostics catalog, ownership doc, runbook, and ownership guard.",
    "Add every new close reason to the source enum label test, diagnostics catalog, runbook, and alert bucket classification when applicable.",
    "Realtime entry modules should record closes through realtime_metrics::record_close_with_store instead of implementing local counters.",
    "Admin UI display changes should keep scripts/test-admin-realtime-health.js in sync.",
    "Diagnostics catalog changes must update server/src/realtime_diagnostics_catalog.snapshot.json and the snapshot test.",
];

pub fn realtime_diagnostics_catalog() -> RealtimeDiagnosticsCatalog {
    RealtimeDiagnosticsCatalog {
        version: "2026-07-16",
        channels: REALTIME_DIAGNOSTIC_CHANNELS,
        close_reasons: REALTIME_DIAGNOSTIC_CLOSE_REASONS,
        change_rules: REALTIME_DIAGNOSTIC_CHANGE_RULES,
    }
}
