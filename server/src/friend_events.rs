use serde::Serialize;
use std::sync::LazyLock;
use tokio::sync::broadcast;

use crate::store::{FriendChatMessage, FriendGroupMessage};

static FRIEND_EVENT_TX: LazyLock<broadcast::Sender<FriendMessagePush>> = LazyLock::new(|| {
    let (tx, _) = broadcast::channel(256);
    tx
});

static GROUP_EVENT_TX: LazyLock<broadcast::Sender<GroupMessagePush>> = LazyLock::new(|| {
    let (tx, _) = broadcast::channel(256);
    tx
});

#[derive(Debug, Clone, Serialize)]
pub struct FriendMessagePush {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    #[serde(rename = "fromUserId")]
    pub from_user_id: String,
    #[serde(rename = "toUserId")]
    pub to_user_id: String,
    #[serde(rename = "messageId")]
    pub message_id: String,
    pub content: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupMessagePush {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    #[serde(rename = "groupId")]
    pub group_id: String,
    #[serde(rename = "fromUserId")]
    pub from_user_id: String,
    #[serde(rename = "messageId")]
    pub message_id: String,
    pub content: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(skip_serializing)]
    pub recipient_user_ids: Vec<String>,
}

impl FriendMessagePush {
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}

impl GroupMessagePush {
    pub fn to_json(&self) -> Option<String> {
        serde_json::to_string(self).ok()
    }
}

pub fn subscribe() -> broadcast::Receiver<FriendMessagePush> {
    FRIEND_EVENT_TX.subscribe()
}

pub fn subscribe_groups() -> broadcast::Receiver<GroupMessagePush> {
    GROUP_EVENT_TX.subscribe()
}

pub fn publish_friend_message(message: &FriendChatMessage) {
    let event = FriendMessagePush {
        event_type: "friend_message",
        from_user_id: message
            .context_user_id
            .clone()
            .unwrap_or_else(|| message.sender_user_id.clone()),
        to_user_id: message.receiver_user_id.clone(),
        message_id: message.id.clone(),
        content: message.content.clone(),
        created_at: message.created_at.clone(),
    };
    let _ = FRIEND_EVENT_TX.send(event);
}

pub fn publish_group_message(message: &FriendGroupMessage, recipient_user_ids: Vec<String>) {
    let event = GroupMessagePush {
        event_type: "group_message",
        group_id: message.group_id.clone(),
        from_user_id: message.sender_user_id.clone(),
        message_id: message.id.clone(),
        content: message.content.clone(),
        created_at: message.created_at.clone(),
        recipient_user_ids,
    };
    let _ = GROUP_EVENT_TX.send(event);
}
