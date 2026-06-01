//! 用户在线状态广播频道
//!
//! 当用户通过 /ws/app 建立认证连接时推送 online；断开时推送 offline。
//! 所有已认证的 WS 连接均可订阅，客户端收到后刷新好友列表即可更新绿点。

use serde::Serialize;
use std::sync::LazyLock;
use tokio::sync::broadcast;

static PRESENCE_TX: LazyLock<broadcast::Sender<PresencePush>> = LazyLock::new(|| {
    let (tx, _) = broadcast::channel(256);
    tx
});

#[derive(Debug, Clone, Serialize)]
pub struct PresencePush {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "isOnline")]
    pub is_online: bool,
}

impl PresencePush {
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}

pub fn subscribe() -> broadcast::Receiver<PresencePush> {
    PRESENCE_TX.subscribe()
}

pub fn publish_online(user_id: String) {
    let _ = PRESENCE_TX.send(PresencePush {
        event_type: "presence",
        user_id,
        is_online: true,
    });
}

pub fn publish_offline(user_id: String) {
    let _ = PRESENCE_TX.send(PresencePush {
        event_type: "presence",
        user_id,
        is_online: false,
    });
}
