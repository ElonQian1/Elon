//! 好友消息已读回执广播频道
//!
//! 当接收方通过 list_friend_messages 打开聊天时，服务端自动标记已读并广播此事件。
//! 发送方的 WS 连接收到后在自己发出的消息气泡下显示"已读"。
//!
//! 事件字段：
//!   - `type`: 固定 "read_receipt"
//!   - `fromUserId`: 阅读消息的用户 ID（即接收方）
//!   - `lastReadAt`: 该用户上次读取时间戳（ISO 8601）

use serde::Serialize;
use std::sync::LazyLock;
use tokio::sync::broadcast;

static READ_RECEIPT_TX: LazyLock<broadcast::Sender<ReadReceiptPush>> = LazyLock::new(|| {
    let (tx, _) = broadcast::channel(256);
    tx
});

#[derive(Debug, Clone, Serialize)]
pub struct ReadReceiptPush {
    /// 固定为 "read_receipt"
    #[serde(rename = "type")]
    pub event_type: &'static str,
    /// 阅读消息的用户 ID（即消息接收方）
    #[serde(rename = "fromUserId")]
    pub from_user_id: String,
    /// 消息原发送方 ID（服务端用于过滤，不发给客户端）
    #[serde(skip)]
    pub to_user_id: String,
    /// 阅读时间戳（ISO 8601），发送方据此判断哪些消息已读
    #[serde(rename = "lastReadAt")]
    pub last_read_at: String,
}

impl ReadReceiptPush {
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}

pub fn subscribe() -> broadcast::Receiver<ReadReceiptPush> {
    READ_RECEIPT_TX.subscribe()
}

/// 通知原发送方：接收方 `reader_user_id` 已读至 `last_read_at`
pub fn publish(reader_user_id: String, sender_user_id: String, last_read_at: String) {
    let _ = READ_RECEIPT_TX.send(ReadReceiptPush {
        event_type: "read_receipt",
        from_user_id: reader_user_id,
        to_user_id: sender_user_id,
        last_read_at,
    });
}
