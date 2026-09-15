use super::{fail, privacy, rich_card::RichCard, Result, SCHEMA};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SnapshotDocument {
    pub schema: String,
    pub provider: String,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub cover_asset_id: Option<String>,
    pub messages: Vec<SnapshotMessage>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SnapshotMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at_ms: i64,
    pub gap_before: bool,
    pub parts: Vec<SnapshotPart>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SnapshotPart {
    #[serde(rename = "type")]
    pub kind: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_block: Option<TextBlock>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<RichCard>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TextBlock {
    pub version: u32,
    pub id: String,
    pub kind: String,
    pub title: String,
    pub language: String,
    pub content: String,
    pub complete: bool,
}
#[derive(Serialize)]
pub(crate) struct SnapshotView {
    pub snapshot_id: String,
    pub group_id: String,
    pub owner_id: String,
    pub owner_name: String,
    pub created_at: String,
    pub document: SnapshotDocument,
}
#[derive(Serialize)]
pub(crate) struct SnapshotCreated {
    pub snapshot_id: String,
    pub group_id: String,
    pub message: crate::store::FriendGroupMessage,
    pub replayed: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SnapshotCard {
    pub schema: String,
    pub snapshot_id: String,
    pub group_id: String,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub sender_name: String,
    #[serde(default)]
    pub cover_asset_id: Option<String>,
    pub provider: String,
    pub message_count: usize,
}
#[derive(Serialize)]
pub(crate) struct SnapshotAsset {
    pub asset_id: String,
    pub mime_type: String,
    pub size_bytes: usize,
}

pub(super) fn opaque(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-.".contains(&c))
}
pub(super) fn language(value: &str) -> bool {
    value.len() <= 32
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_+.#-".contains(&c))
}
pub(super) fn text(value: &str, max: usize) -> Result<()> {
    if value.chars().count() > max {
        return Err(fail(413, "Snapshot text limit exceeded"));
    }
    privacy::validate_text(value)
}

impl SnapshotDocument {
    pub(super) fn validate(&self) -> Result<String> {
        if self.schema != SCHEMA
            || self.provider != "chatgpt"
            || self.title.trim().is_empty()
            || self.messages.is_empty()
            || self.messages.len() > 200
        {
            return Err(fail(
                400,
                "Invalid snapshot schema, provider, title or message count",
            ));
        }
        let json = serde_json::to_string(self)?;
        if json.len() > 2 * 1024 * 1024 {
            return Err(fail(413, "Snapshot JSON exceeds 2 MiB"));
        }
        text(&self.title, 120)?;
        text(&self.summary, 300)?;
        if self
            .cover_asset_id
            .as_deref()
            .is_some_and(|id| !self.asset_ids().contains(id))
        {
            return Err(fail(400, "Cover must reference a selected image asset"));
        }
        let mut ids = BTreeSet::new();
        let mut blocks = 0;
        let mut images = 0;
        let mut text_count = 0;
        for message in &self.messages {
            if !opaque(&message.id)
                || !ids.insert(&message.id)
                || !matches!(message.role.as_str(), "user" | "assistant")
                || !(0..=253_402_300_799_999).contains(&message.created_at_ms)
                || (message.content.trim().is_empty() && message.parts.is_empty())
            {
                return Err(fail(400, "Invalid snapshot message"));
            }
            text(&message.content, 120_000)?;
            text_count += message.content.chars().count();
            blocks += message.parts.len();
            for part in &message.parts {
                part.validate()?;
                images += usize::from(part.kind == "image");
                if let Some(block) = &part.text_block {
                    text_count += block.content.chars().count();
                }
            }
        }
        if blocks > 256 || images > 12 || text_count > 500_000 {
            return Err(fail(
                413,
                "Snapshot exceeds 256 parts, 12 images or 500000 text characters",
            ));
        }
        Ok(json)
    }

    pub(super) fn asset_ids(&self) -> BTreeSet<&str> {
        self.messages
            .iter()
            .flat_map(|m| &m.parts)
            .filter_map(|p| p.asset_id.as_deref())
            .collect()
    }

    pub(super) fn assign_public_ids(&mut self) {
        for message in &mut self.messages {
            message.id = super::new_id("shared_message");
            for part in &mut message.parts {
                if part.kind == "image" {
                    part.media_type = Some("image/png".into());
                }
                if let Some(block) = &mut part.text_block {
                    block.id = super::new_id("shared_block");
                }
            }
        }
    }
}

impl SnapshotPart {
    fn validate(&self) -> Result<()> {
        text(&self.label, 180)?;
        if let Some(caption) = &self.caption {
            text(caption, 1000)?;
        }
        if self.language.as_deref().is_some_and(|v| !language(v)) {
            return Err(fail(400, "Invalid part language"));
        }
        if self.kind != "image" && (self.asset_id.is_some() || self.media_type.is_some()) {
            return Err(fail(400, "Only image parts may reference assets"));
        }
        if self.kind != "rich_card" && self.card.is_some() {
            return Err(fail(400, "Unexpected rich card"));
        }
        if !matches!(self.kind.as_str(), "code" | "writing_block") && self.text_block.is_some() {
            return Err(fail(400, "Unexpected text block"));
        }
        match self.kind.as_str() {
            "image" => {
                if !self
                    .asset_id
                    .as_deref()
                    .is_some_and(|v| opaque(v) && v.starts_with("article_media_"))
                    || self
                        .media_type
                        .as_deref()
                        .is_some_and(|v| !matches!(v, "image/png" | "image/jpeg" | "image/webp"))
                {
                    return Err(fail(400, "Image requires an uploaded asset"));
                }
            }
            "code" | "writing_block" => {
                let block = self
                    .text_block
                    .as_ref()
                    .ok_or_else(|| fail(400, "Complete text_block required"))?;
                if block.version != 1
                    || !block.complete
                    || !opaque(&block.id)
                    || !language(&block.language)
                    || block.kind
                        != if self.kind == "code" {
                            "code"
                        } else {
                            "writing"
                        }
                {
                    return Err(fail(400, "Invalid or incomplete text block"));
                }
                text(&block.title, 160)?;
                text(&block.content, 120_000)?;
            }
            "rich_card" => self
                .card
                .as_ref()
                .ok_or_else(|| fail(400, "Rich card required"))?
                .validate()?,
            "table" | "math" | "citation" | "unavailable" => {
                if self.label.trim().is_empty() {
                    return Err(fail(400, "Part label required"));
                }
            }
            _ => {
                return Err(fail(
                    400,
                    "Unsupported rich part; use an explicit unavailable part",
                ))
            }
        }
        Ok(())
    }
}
