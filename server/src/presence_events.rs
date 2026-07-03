//! 用户在线状态广播频道
//!
//! 当用户通过 /ws/app 建立认证连接时推送 online；断开时推送 offline。
//! 用户修改展示状态时也会推送具体状态，客户端可直接更新成员栏。

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(rename = "customStatus", skip_serializing_if = "Option::is_none")]
    pub custom_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity: Option<String>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
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
        status: None,
        custom_status: None,
        activity: None,
        updated_at: None,
    });
}

pub fn publish_offline(user_id: String) {
    let _ = PRESENCE_TX.send(PresencePush {
        event_type: "presence",
        user_id,
        is_online: false,
        status: Some("offline".to_string()),
        custom_status: None,
        activity: None,
        updated_at: None,
    });
}

pub fn publish_settings(
    user_id: String,
    connected: bool,
    status: &str,
    custom_status: Option<String>,
    activity: Option<String>,
    updated_at: Option<String>,
) {
    let (is_online, status, custom_status, activity) =
        effective_presence(connected, status, custom_status, activity);
    let _ = PRESENCE_TX.send(PresencePush {
        event_type: "presence",
        user_id,
        is_online,
        status: Some(status),
        custom_status,
        activity,
        updated_at,
    });
}

pub(crate) fn effective_presence(
    connected: bool,
    status: &str,
    custom_status: Option<String>,
    activity: Option<String>,
) -> (bool, String, Option<String>, Option<String>) {
    let configured = status.trim().to_ascii_lowercase();
    if !connected || configured == "invisible" {
        return (false, "offline".to_string(), None, None);
    }
    let visible_status = match configured.as_str() {
        "idle" | "dnd" | "online" => configured,
        _ => "online".to_string(),
    };
    (true, visible_status, custom_status, activity)
}

#[cfg(test)]
mod tests {
    use super::effective_presence;

    #[test]
    fn effective_presence_hides_invisible_details() {
        let (is_online, status, custom_status, activity) = effective_presence(
            true,
            "invisible",
            Some("coding".to_string()),
            Some("reviewing".to_string()),
        );
        assert!(!is_online);
        assert_eq!(status, "offline");
        assert!(custom_status.is_none());
        assert!(activity.is_none());
    }

    #[test]
    fn effective_presence_keeps_visible_status_details() {
        let (is_online, status, custom_status, activity) = effective_presence(
            true,
            "dnd",
            Some("coding".to_string()),
            Some("reviewing".to_string()),
        );
        assert!(is_online);
        assert_eq!(status, "dnd");
        assert_eq!(custom_status.as_deref(), Some("coding"));
        assert_eq!(activity.as_deref(), Some("reviewing"));
    }
}
