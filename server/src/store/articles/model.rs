use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArticleDocument {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub cover: Option<String>,
    #[serde(default)]
    pub blocks: Vec<ArticleBlock>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ArticleBlock {
    Paragraph {
        text: String,
    },
    Heading {
        text: String,
    },
    Quote {
        text: String,
    },
    Image {
        media_id: String,
        #[serde(default)]
        caption: String,
    },
}
impl ArticleDocument {
    pub(super) fn media_ids(&self) -> BTreeSet<String> {
        self.cover
            .iter()
            .cloned()
            .chain(self.blocks.iter().filter_map(|b| match b {
                ArticleBlock::Image { media_id, .. } => Some(media_id.clone()),
                _ => None,
            }))
            .collect()
    }
    pub(super) fn validate(&self, conn: &Connection, owner: &str, publish: bool) -> Result<()> {
        if self.title.chars().count() > 120
            || self.summary.chars().count() > 300
            || self.blocks.len() > 120
        {
            return Err(fail(400, "标题最多120字，摘要最多300字，正文最多120个段落"));
        }
        let mut text_count = 0;
        let mut image_count = 0;
        let mut has_body = false;
        for block in &self.blocks {
            let text = match block {
                ArticleBlock::Paragraph { text }
                | ArticleBlock::Heading { text }
                | ArticleBlock::Quote { text } => text,
                ArticleBlock::Image { caption, .. } => {
                    image_count += 1;
                    has_body = true;
                    caption
                }
            };
            text_count += text.chars().count();
            has_body |= !text.trim().is_empty();
        }
        if text_count > 50_000 {
            return Err(fail(400, "文章正文最多50000字"));
        }
        if image_count > 12 {
            return Err(fail(400, "正文最多12个图片块"));
        }
        if publish && (self.title.trim().is_empty() || !has_body) {
            return Err(fail(400, "发布前请填写标题和正文"));
        }
        let media = self.media_ids();
        if media.len() > 12 {
            return Err(fail(400, "每篇文章最多12张图片（含封面）"));
        }
        for id in media {
            let valid: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM social_content_media WHERE id=?1 AND owner_id=?2)",
                params![id, owner],
                |r| r.get(0),
            )?;
            if !valid {
                return Err(fail(400, "图片不属于当前作者或已不存在"));
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ArticleCard {
    pub id: String,
    pub revision: i64,
    pub title: String,
    pub summary: String,
    pub author_id: String,
    pub author_name: String,
    pub cover_data_url: Option<String>,
    pub status: String,
    pub updated_at: String,
}
#[derive(Debug, Serialize)]
pub(crate) struct ArticleView {
    #[serde(flatten)]
    pub card: ArticleCard,
    pub document: ArticleDocument,
    pub media: std::collections::BTreeMap<String, String>,
}
#[derive(Debug, Serialize)]
pub(crate) struct ArticlePage {
    pub items: Vec<ArticleCard>,
    pub next_offset: Option<i64>,
}
#[derive(Debug, Serialize)]
pub(crate) struct ArticlePublishResult {
    pub article_id: String,
    pub revision: i64,
    pub messages: Vec<super::super::FriendGroupMessage>,
    pub already_shared_groups: Vec<String>,
}
