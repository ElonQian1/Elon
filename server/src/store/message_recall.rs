use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};

pub(super) const RECALL_NOTICE_SELF: &str = "你撤回了一条消息";
pub(super) const RECALL_NOTICE_OTHER: &str = "对方撤回了一条消息";

const MESSAGE_RECALL_WINDOW: Duration = Duration::seconds(60);

pub(super) fn ensure_message_recall_allowed(created_at: &str) -> Result<()> {
    let created_at = DateTime::parse_from_rfc3339(created_at)
        .map_err(|_| anyhow!("消息时间异常，暂时不能撤回"))?
        .with_timezone(&Utc);
    if Utc::now().signed_duration_since(created_at) > MESSAGE_RECALL_WINDOW {
        return Err(anyhow!("只能撤回 1 分钟内发送的消息"));
    }
    Ok(())
}

pub(super) fn recalled_content(content: String, recalled_at: Option<&str>) -> String {
    if recalled_at.is_some() {
        String::new()
    } else {
        content
    }
}

pub(super) fn recall_preview_for_viewer(
    recalled_at: Option<&str>,
    recalled_by: Option<&str>,
    viewer_user_id: &str,
) -> Option<String> {
    recalled_at.map(|_| {
        if recalled_by == Some(viewer_user_id) {
            RECALL_NOTICE_SELF.to_string()
        } else {
            RECALL_NOTICE_OTHER.to_string()
        }
    })
}
