use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Share {
    pub task_id: String,
    pub account_scope: String,
    pub title: String,
    pub consent: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Sync {
    pub account_scope: String,
    pub task_id: String,
    pub state: String,
    pub update: Option<Update>,
}
#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct Update {
    pub id: String,
    pub created_at: String,
    pub content: String,
}
pub(super) fn opaque(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
}
pub(super) fn scope(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
pub(super) fn text(value: &str, max: usize) -> Result<()> {
    ensure!(
        !value.trim().is_empty() && value.len() <= max,
        "invalid text"
    );
    crate::store::articles::snapshots::validate_shared_text(value)
}
impl Share {
    pub(super) fn validate(&self) -> Result<()> {
        ensure!(
            self.consent && opaque(&self.task_id) && scope(&self.account_scope),
            "consent required"
        );
        text(&self.title, 2048)
    }
}
impl Sync {
    pub(super) fn validate(&self) -> Result<()> {
        ensure!(
            opaque(&self.task_id) && scope(&self.account_scope),
            "invalid source"
        );
        ensure!(
            [
                "ready",
                "no_update",
                "requires_action",
                "missing",
                "unavailable",
                "auth_required"
            ]
            .contains(&self.state.as_str()),
            "invalid state"
        );
        ensure!(
            self.update.is_none() || self.state == "ready",
            "invalid update state"
        );
        if let Some(update) = &self.update {
            ensure!(opaque(&update.id), "invalid update id");
            let at = chrono::DateTime::parse_from_rfc3339(&update.created_at)?;
            ensure!(
                at.timestamp() <= chrono::Utc::now().timestamp() + 300 && at.timestamp() > 0,
                "invalid update time"
            );
            text(&update.content, 512 * 1024)?;
        }
        Ok(())
    }
}
