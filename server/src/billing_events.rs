//! 计费事件广播频道
//!
//! 目前只有一种事件：`low_balance`（余额低于阈值警告）。
//! 通过 global_ws.rs 的 tokio::select! 推送给已认证的用户连接。

use serde::Serialize;
use std::sync::LazyLock;
use tokio::sync::broadcast;

static BILLING_TX: LazyLock<broadcast::Sender<BillingPush>> = LazyLock::new(|| {
    let (tx, _) = broadcast::channel(256);
    tx
});

/// 计费推送消息（发给 APK）。
#[derive(Debug, Clone, Serialize)]
pub struct BillingPush {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    #[serde(rename = "userId")]
    pub user_id: String,
    /// 当前余额（分）
    #[serde(rename = "balanceFen")]
    pub balance_fen: i64,
}

impl BillingPush {
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}

pub fn subscribe() -> broadcast::Receiver<BillingPush> {
    BILLING_TX.subscribe()
}

/// 余额低于阈值时广播给相应用户（用户在 WS 处理器中按 user_id 过滤）。
pub fn publish_low_balance(user_id: String, balance_fen: i64) {
    let _ = BILLING_TX.send(BillingPush {
        event_type: "low_balance",
        user_id,
        balance_fen,
    });
}
