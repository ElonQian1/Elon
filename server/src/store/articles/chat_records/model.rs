use super::{fail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SCHEMA: &str = "chat_record_bundle_v1";
pub(crate) const PREFIX: &str = "【一龙聊天记录】\n";
pub(crate) const MAX_DOCUMENT: usize = 2 * 1024 * 1024;
pub(crate) const MAX_ASSET: usize = 12 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Document {
    pub schema: String,
    pub source: String,
    pub title: String,
    pub raw_text: String,
    pub messages: Vec<Record>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub sender: String,
    pub time: String,
    pub kind: String,
    pub text: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub asset_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Card {
    pub schema: String,
    pub record_id: String,
    pub group_id: String,
    pub title: String,
    pub summary: String,
    pub message_count: usize,
    pub total_count: usize,
}

#[derive(Serialize)]
pub(crate) struct View {
    pub card: Card,
    pub owner_id: String,
    pub document: Document,
}

#[derive(Serialize)]
pub(crate) struct Created {
    pub card: Card,
    pub message: crate::store::FriendGroupMessage,
    pub replayed: bool,
}

#[derive(Serialize)]
pub(crate) struct Asset {
    pub asset_id: String,
    pub mime_type: String,
    pub size_bytes: usize,
}

pub(super) fn opaque(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || b"_-".contains(&v))
}

impl Document {
    pub(super) fn validate(&self) -> Result<String> {
        let json = serde_json::to_string(self)?;
        if json.len() > MAX_DOCUMENT || self.raw_text.len() > 1024 * 1024 {
            return Err(fail(413, "聊天记录超过大小限制"));
        }
        if self.schema != SCHEMA
            || self.source != "wechat"
            || self.title.trim().is_empty()
            || self.title.chars().count() > 120
            || self.messages.is_empty()
            || self.messages.len() > 2000
            || self.warnings.len() > 100
            || self.warnings.iter().any(|s| s.chars().count() > 300)
        {
            return Err(fail(400, "聊天记录格式不受支持"));
        }
        let mut seen: BTreeMap<&str, (&str, usize)> = BTreeMap::new();
        for row in &self.messages {
            if !opaque(&row.id)
                || seen.contains_key(row.id.as_str())
                || row.sender.chars().count() > 200
                || row.time.len() > 100
                || row.text.len() > 256 * 1024
                || row.filename.chars().count() > 240
                || !matches!(
                    row.kind.as_str(),
                    "text"
                        | "forward"
                        | "image"
                        | "video"
                        | "audio"
                        | "file"
                        | "link"
                        | "channels"
                        | "unknown"
                )
            {
                return Err(fail(400, "聊天记录消息无效"));
            }
            let depth = match row.parent_id.as_deref() {
                None => 0,
                Some(parent) => match seen.get(parent) {
                    Some(("forward", depth)) => depth + 1,
                    _ => return Err(fail(400, "聊天记录层级无效")),
                },
            };
            if depth > 8 {
                return Err(fail(413, "聊天记录嵌套过深"));
            }
            if let Some(asset) = &row.asset_id {
                if !opaque(asset)
                    || !matches!(row.kind.as_str(), "image" | "video" | "audio" | "file")
                {
                    return Err(fail(400, "附件引用无效"));
                }
            }
            seen.insert(&row.id, (&row.kind, depth));
        }
        if self.asset_ids().len() > 64 {
            return Err(fail(413, "附件数量超过限制"));
        }
        Ok(json)
    }

    pub(super) fn asset_ids(&self) -> BTreeSet<&str> {
        self.messages
            .iter()
            .filter_map(|r| r.asset_id.as_deref())
            .collect()
    }

    pub(super) fn card(&self, id: &str, group: &str) -> Card {
        Card {
            schema: SCHEMA.into(),
            record_id: id.into(),
            group_id: group.into(),
            title: self.title.clone(),
            message_count: self
                .messages
                .iter()
                .filter(|r| r.parent_id.is_none())
                .count(),
            total_count: self.messages.len(),
            summary: self
                .messages
                .iter()
                .filter(|r| r.parent_id.is_none())
                .take(3)
                .map(|r| {
                    format!(
                        "{}：{}",
                        r.sender,
                        r.text
                            .replace('\n', " ")
                            .chars()
                            .take(65)
                            .collect::<String>()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

pub(crate) fn message_preview(content: &str) -> Option<String> {
    let card: Card = serde_json::from_str(content.strip_prefix(PREFIX)?).ok()?;
    (card.schema == SCHEMA && opaque(&card.record_id)).then(|| format!("[聊天记录] {}", card.title))
}
