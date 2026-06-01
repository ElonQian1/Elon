//! 好友正在输入广播频道
//!
//! 当用户通过 /ws/app 连接发送 `{"type":"typing","to_user_id":"..."}` 消息时触发。
//! 服务端将事件广播给目标用户的所有已认证 WS 连接。
//! 客户端收到后在聊天页面显示"正在输入..."提示，3 秒无新事件后自动隐藏。

use serde::Serialize;
use std::sync::LazyLock;
use tokio::sync::broadcast;

static TYPING_TX: LazyLock<broadcast::Sender<TypingPush>> = LazyLock::new(|| {
    let (tx, _) = broadcast::channel(256);
    tx
});

#[derive(Debug, Clone, Serialize)]
pub struct TypingPush {
    /// 固定为 "typing"
    #[serde(rename = "type")]
    pub event_type: &'static str,
    /// 正在输入的用户 ID
    #[serde(rename = "fromUserId")]
    pub from_user_id: String,
    /// 消息目标用户 ID（服务端用于过滤，不发给客户端）
    #[serde(skip)]
    pub to_user_id: String,
}

impl TypingPush {
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}

pub fn subscribe() -> broadcast::Receiver<TypingPush> {
    TYPING_TX.subscribe()
}

pub fn publish(from_user_id: String, to_user_id: String) {
    let _ = TYPING_TX.send(TypingPush {
        event_type: "typing",
        from_user_id,
        to_user_id,
    });
}
