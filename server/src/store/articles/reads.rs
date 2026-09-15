use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::collections::BTreeMap;

pub(super) fn card(
    conn: &Connection,
    id: &str,
    revision: i64,
    owner: &str,
    status: &str,
    time: String,
    doc: &ArticleDocument,
) -> Result<ArticleCard> {
    let cover_data_url = doc
        .cover
        .as_ref()
        .map(|media| {
            conn.query_row(
                "SELECT thumbnail FROM social_content_media WHERE id=?1 AND owner_id=?2",
                params![media, owner],
                |r| r.get(0),
            )
        })
        .transpose()?;
    Ok(ArticleCard {
        id: id.into(),
        revision,
        title: doc.title.clone(),
        summary: doc.summary.clone(),
        author_id: owner.into(),
        author_name: author_name(conn, owner)?,
        cover_data_url,
        status: status.into(),
        updated_at: time,
    })
}
fn view(
    conn: &Connection,
    card: ArticleCard,
    document: ArticleDocument,
    compact: bool,
) -> Result<ArticleView> {
    let mut media = BTreeMap::new();
    if !compact {
        for id in document.media_ids() {
            let (mime, bytes): (String, Vec<u8>) = conn.query_row(
                "SELECT mime_type,bytes FROM social_content_media WHERE id=?1 AND owner_id=?2",
                params![id, card.author_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            media.insert(id, format!("data:{mime};base64,{}", STANDARD.encode(bytes)));
        }
    }
    let document = if compact {
        ArticleDocument {
            blocks: vec![],
            ..document
        }
    } else {
        document
    };
    Ok(ArticleView {
        card,
        document,
        media,
    })
}
pub(super) fn draft_view(conn: &Connection, user: &str, id: &str) -> Result<ArticleView> {
    let head = owned(conn, user, id)?;
    let time = conn.query_row(
        "SELECT updated_at FROM social_contents WHERE id=?1",
        [id],
        |r| r.get(0),
    )?;
    let card = card(
        conn,
        id,
        head.version,
        user,
        &head.status,
        time,
        &head.document,
    )?;
    view(conn, card, head.document, false)
}
pub(super) fn published_view(
    conn: &Connection,
    user: &str,
    id: &str,
    revision: i64,
    compact: bool,
) -> Result<ArticleView> {
    let head = head(conn, id)?;
    if head.status != "published" {
        return Err(fail(404, "文章已撤下或不可访问"));
    }
    let allowed: bool = head.owner == user
        || conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM social_content_distributions d
         JOIN friend_group_members gm ON gm.group_id=d.target_id AND gm.user_id=?3
         JOIN friend_group_messages m ON m.id=d.message_id AND m.recalled_at IS NULL
         WHERE d.content_id=?1 AND d.revision=?2 AND d.target_kind='group')",
            params![id, revision, user],
            |r| r.get(0),
        )?;
    if !allowed {
        return Err(fail(404, "文章不存在或你没有阅读权限"));
    }
    let row:Option<(String,String)>=conn.query_row("SELECT document_json,published_at FROM social_content_revisions WHERE content_id=?1 AND revision=?2",params![id,revision],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    let (json, time) = row.ok_or_else(|| fail(404, "此文章版本不存在"))?;
    let document: ArticleDocument = serde_json::from_str(&json)?;
    let card = card(
        conn,
        id,
        revision,
        &head.owner,
        "published",
        time,
        &document,
    )?;
    view(conn, card, document, compact)
}
impl Store {
    pub(crate) fn read_article(
        &self,
        user: &str,
        id: &str,
        revision: i64,
        compact: bool,
    ) -> Result<ArticleView> {
        let conn = self.conn()?;
        published_view(&conn, user, id, revision, compact)
    }
    pub(crate) fn list_articles(
        &self,
        user: &str,
        group: Option<&str>,
        offset: i64,
    ) -> Result<ArticlePage> {
        let conn = self.conn()?;
        let offset = offset.clamp(0, 100_000);
        let rows: Vec<(String, i64)> = if let Some(group) = group {
            member(&conn, user, group)?;
            let mut statement = conn.prepare(
                "SELECT d.content_id,MAX(d.revision) FROM social_content_distributions d
                JOIN social_contents c ON c.id=d.content_id AND c.status='published' AND c.kind='article'
                JOIN friend_group_messages m ON m.id=d.message_id AND m.recalled_at IS NULL
                WHERE d.target_kind='group' AND d.target_id=?1 GROUP BY d.content_id
                ORDER BY MAX(d.created_at) DESC,d.content_id LIMIT 21 OFFSET ?2",
            )?;
            let rows = statement
                .query_map(params![group, offset], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        } else {
            let mut statement=conn.prepare("SELECT id,edit_version FROM social_contents WHERE owner_id=?1 AND kind='article' ORDER BY updated_at DESC,id LIMIT 21 OFFSET ?2")?;
            let rows = statement
                .query_map(params![user, offset], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        };
        let next_offset = (rows.len() > 20).then_some(offset + 20);
        let mut items = vec![];
        for (id, revision) in rows.into_iter().take(20) {
            if group.is_some() {
                items.push(published_view(&conn, user, &id, revision, true)?.card);
            } else {
                let head = owned(&conn, user, &id)?;
                let time = conn.query_row(
                    "SELECT updated_at FROM social_contents WHERE id=?1",
                    [&id],
                    |r| r.get(0),
                )?;
                items.push(card(
                    &conn,
                    &id,
                    revision,
                    user,
                    &head.status,
                    time,
                    &head.document,
                )?);
            }
        }
        Ok(ArticlePage { items, next_offset })
    }
}
