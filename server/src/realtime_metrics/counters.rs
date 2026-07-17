use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use serde::Serialize;

use crate::store::Store;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealtimeChannel {
    AppNotify,
    GlobalApp,
    HomecliAgent,
    PeerRelay,
    ProjectWs,
    VoiceRealtimeChat,
    VoiceTranscribe,
    VoiceVirtualMic,
}

impl RealtimeChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AppNotify => "app_notify",
            Self::GlobalApp => "global_app",
            Self::HomecliAgent => "homecli_agent",
            Self::PeerRelay => "peer_relay",
            Self::ProjectWs => "project_ws",
            Self::VoiceRealtimeChat => "voice_realtime_chat",
            Self::VoiceTranscribe => "voice_transcribe",
            Self::VoiceVirtualMic => "voice_virtual_mic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RealtimeCloseMetricSnapshot {
    pub channel: String,
    pub close_reason: String,
    pub count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RealtimeCloseMetricKey {
    channel: &'static str,
    close_reason: &'static str,
}

static CLOSE_COUNTERS: OnceLock<Mutex<HashMap<RealtimeCloseMetricKey, u64>>> = OnceLock::new();

fn close_counters() -> &'static Mutex<HashMap<RealtimeCloseMetricKey, u64>> {
    CLOSE_COUNTERS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
pub fn record_close(channel: RealtimeChannel, close_reason: &'static str) -> u64 {
    record_close_inner(None, channel, close_reason)
}

pub fn record_close_with_store(
    store: &Store,
    channel: RealtimeChannel,
    close_reason: &'static str,
) -> u64 {
    record_close_inner(Some(store), channel, close_reason)
}

fn record_close_inner(
    store: Option<&Store>,
    channel: RealtimeChannel,
    close_reason: &'static str,
) -> u64 {
    let channel = channel.as_str();
    let key = RealtimeCloseMetricKey {
        channel,
        close_reason,
    };
    let count = {
        let mut counters = close_counters()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let count = counters.entry(key).or_default();
        *count += 1;
        *count
    };

    if let Some(store) = store {
        if let Err(error) = store.record_realtime_close_event(channel, close_reason) {
            tracing::warn!(
                target: "realtime_metrics",
                channel,
                close_reason,
                error = %error,
                "failed to persist realtime close event"
            );
        }
    }

    tracing::info!(
        target: "realtime_metrics",
        transport = "websocket",
        channel,
        close_reason,
        count,
        "realtime connection closed"
    );

    count
}

pub fn close_metric_snapshot() -> Vec<RealtimeCloseMetricSnapshot> {
    let counters = close_counters()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut rows: Vec<_> = counters
        .iter()
        .map(|(key, count)| RealtimeCloseMetricSnapshot {
            channel: key.channel.to_string(),
            close_reason: key.close_reason.to_string(),
            count: *count,
        })
        .collect();
    rows.sort_by(|a, b| {
        a.channel
            .cmp(&b.channel)
            .then_with(|| a.close_reason.cmp(&b.close_reason))
    });
    rows
}

#[cfg(test)]
pub fn reset_for_tests() {
    close_counters()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clear();
}
