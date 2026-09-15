use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Mode {
    Article,
    Text,
    Images,
    Video,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Selection {
    pub article_id: String,
    pub version: i64,
    pub mode: Mode,
    #[serde(default)]
    pub media_ids: Vec<String>,
    #[serde(default)]
    pub cover_id: Option<String>,
    #[serde(default)]
    pub video_id: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Payload {
    pub selection: Selection,
    pub title: String,
    pub text: String,
    pub warnings: Vec<String>,
}
#[derive(Serialize)]
pub(crate) struct Preview {
    #[serde(flatten)]
    pub payload: Payload,
    pub preview_hash: String,
    pub generation: i64,
    pub account_label: String,
    pub media: std::collections::BTreeMap<String, String>,
}
#[derive(Serialize)]
pub(crate) struct Account {
    pub bound: bool,
    pub label: String,
    pub masked_key: String,
    pub generation: i64,
    pub verified_at: Option<i64>,
    pub creator_url: &'static str,
}
#[derive(Clone, Serialize)]
pub(crate) struct Job {
    pub id: String,
    pub article_id: String,
    pub revision: i64,
    pub title: String,
    pub mode: Mode,
    pub status: String,
    pub scheduled_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub attempts: i64,
    pub message: String,
    pub post_url: Option<String>,
}
pub(super) struct Work {
    pub id: String,
    pub owner: String,
    pub generation: i64,
    pub payload: Payload,
}

pub(super) fn conversion(conn: &Connection, owner: &str, selection: &Selection) -> Result<Payload> {
    let h = owned(conn, owner, &selection.article_id)?;
    if h.version != selection.version || h.status == "withdrawn" {
        return Err(fail(409, "文章已变化或撤下，请重新预览"));
    }
    h.document.validate(conn, owner, true)?;
    let mut text = vec![];
    let mut image_count = 0;
    for block in &h.document.blocks {
        match block {
            ArticleBlock::Paragraph { text: t }
            | ArticleBlock::Heading { text: t }
            | ArticleBlock::Quote { text: t } => {
                if !t.trim().is_empty() {
                    text.push(t.clone());
                }
            }
            ArticleBlock::Image { caption, .. } => {
                image_count += 1;
                if !caption.trim().is_empty() {
                    text.push(caption.clone());
                }
            }
        }
    }
    let mut warnings = vec![];
    if image_count > 0 {
        warnings.push(format!("原文有{image_count}个插图块。币安正文仅发送文字和图片说明；只上传下方选定的封面或动态图片。"));
    }
    if !h.document.summary.is_empty() {
        warnings.push("摘要仅用于一龙卡片，不另加到币安正文中。".into());
    }
    let title = h.document.title;
    let text = if matches!(selection.mode, Mode::Article) {
        text.join("\n\n")
    } else {
        std::iter::once(title.clone())
            .chain(text)
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    if text.trim().is_empty() {
        return Err(fail(400, "币安版本需要可发送的文字正文"));
    }
    if selection.media_ids.len() > 4
        || selection
            .media_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != selection.media_ids.len()
    {
        return Err(fail(400, "图片动态请选择1至4张不同图片"));
    }
    match selection.mode {
        Mode::Article if !selection.media_ids.is_empty() || selection.video_id.is_some() => {
            return Err(fail(400, "长文仅支持单张封面"))
        }
        Mode::Text
            if !selection.media_ids.is_empty()
                || selection.cover_id.is_some()
                || selection.video_id.is_some() =>
        {
            return Err(fail(400, "文字动态不能携带媒体"))
        }
        Mode::Images
            if selection.media_ids.is_empty()
                || selection.cover_id.is_some()
                || selection.video_id.is_some() =>
        {
            return Err(fail(400, "图片动态请选择1至4张图片"))
        }
        Mode::Video
            if selection.video_id.is_none()
                || !selection.media_ids.is_empty()
                || selection.cover_id.is_some() =>
        {
            return Err(fail(400, "视频动态请单独选择视频，使用视频首帧封面"))
        }
        _ => {}
    }
    for id in selection.media_ids.iter().chain(selection.cover_id.iter()) {
        media::image_bytes(conn, owner, id)?;
    }
    if let Some(id) = &selection.video_id {
        media::video_metadata(conn, owner, id)?;
    }
    Ok(Payload {
        selection: selection.clone(),
        title,
        text,
        warnings,
    })
}
pub(super) fn preview_hash(payload: &Payload, generation: i64) -> Result<String> {
    Ok(digest(&format!(
        "{generation}:{}",
        serde_json::to_string(payload)?
    )))
}

impl Store {
    pub(crate) fn square_preview(&self, owner: &str, selection: Selection) -> Result<Preview> {
        let conn = self.conn()?;
        let account = account::read(&conn, owner)?;
        if !account.bound {
            return Err(fail(409, "请先绑定币安广场发帖凭证"));
        }
        let payload = conversion(&conn, owner, &selection)?;
        let mut media = std::collections::BTreeMap::new();
        for id in selection.media_ids.iter().chain(selection.cover_id.iter()) {
            let (mime, bytes) = media::image_bytes(&conn, owner, id)?;
            use base64::Engine;
            media.insert(
                id.clone(),
                format!(
                    "data:{mime};base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                ),
            );
        }
        if let Some(id) = &selection.video_id {
            let (_, cover) = media::video_metadata(&conn, owner, id)?;
            let (mime, bytes) = media::image_bytes(&conn, owner, &cover)?;
            use base64::Engine;
            media.insert(
                cover,
                format!(
                    "data:{mime};base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                ),
            );
        }
        Ok(Preview {
            preview_hash: preview_hash(&payload, account.generation)?,
            payload,
            generation: account.generation,
            account_label: account.label,
            media,
        })
    }
}
