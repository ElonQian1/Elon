//! Public share links: one unguessable token per published revision, opt-in by the author.
//! A token is a `public` distribution, so withdrawing the article also closes the page.
use super::*;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ArticleShare {
    pub token: String,
    pub revision: i64,
    pub path: String,
}

fn share_path(token: &str) -> String {
    format!("/a/{token}")
}

pub(crate) fn valid_token(value: &str) -> bool {
    value.len() <= 64
        && value.starts_with("s_")
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

impl Store {
    /// Publishes `version` (if not already) and returns its public link, reusing an existing one.
    pub(crate) fn create_article_share(
        &self,
        user: &str,
        id: &str,
        version: i64,
    ) -> Result<ArticleShare> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let head = owned(&tx, user, id)?;
        if head.status == "withdrawn" {
            return Err(fail(409, "文章已撤下，不能公开分享"));
        }
        if head.version != version {
            return Err(fail(409, "草稿版本已变化，请重新预览后分享"));
        }
        head.document.validate(&tx, user, true)?;
        let time = now();
        tx.execute(
            "INSERT OR IGNORE INTO social_content_revisions VALUES (?1,?2,?3,?4)",
            params![id, version, serde_json::to_string(&head.document)?, time],
        )?;
        let existing: Option<String> = tx
            .query_row(
                "SELECT target_id FROM social_content_distributions WHERE content_id=?1 AND revision=?2 AND target_kind='public'",
                params![id, version],
                |r| r.get(0),
            )
            .optional()?;
        let token = match existing {
            Some(token) => token,
            None => {
                let token = new_id("s");
                tx.execute(
                    "INSERT INTO social_content_distributions VALUES (?1,?2,'public',?3,NULL,?4)",
                    params![id, version, token, time],
                )?;
                token
            }
        };
        tx.execute(
            "UPDATE social_contents SET status='published',updated_at=?2 WHERE id=?1",
            params![id, time],
        )?;
        tx.commit()?;
        Ok(ArticleShare {
            path: share_path(&token),
            token,
            revision: version,
        })
    }

    /// The author's current public link (latest shared revision), if any.
    pub(crate) fn article_share(&self, user: &str, id: &str) -> Result<Option<ArticleShare>> {
        let conn = self.conn()?;
        owned(&conn, user, id)?;
        let row: Option<(String, i64)> = conn
            .query_row(
                "SELECT target_id,revision FROM social_content_distributions WHERE content_id=?1 AND target_kind='public' ORDER BY revision DESC LIMIT 1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        Ok(row.map(|(token, revision)| ArticleShare {
            path: share_path(&token),
            token,
            revision,
        }))
    }

    /// Closes every public link of the article; group readers are unaffected.
    pub(crate) fn revoke_article_share(&self, user: &str, id: &str) -> Result<()> {
        let conn = self.conn()?;
        owned(&conn, user, id)?;
        conn.execute(
            "DELETE FROM social_content_distributions WHERE content_id=?1 AND target_kind='public'",
            [id],
        )?;
        Ok(())
    }

    /// Anonymous read of a shared revision; the document and media belong to the author.
    pub(crate) fn public_article(&self, token: &str) -> Result<ArticleView> {
        let conn = self.conn()?;
        let (id, revision) = public_target(&conn, token)?;
        let head = head(&conn, &id)?;
        if head.status != "published" {
            return Err(fail(404, "文章已撤下"));
        }
        let row: Option<(String, String)> = conn
            .query_row(
                "SELECT document_json,published_at FROM social_content_revisions WHERE content_id=?1 AND revision=?2",
                params![id, revision],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let (json, time) = row.ok_or_else(|| fail(404, "文章不存在"))?;
        let document: ArticleDocument = serde_json::from_str(&json)?;
        let card = reads::card(
            &conn,
            &id,
            revision,
            &head.owner,
            "published",
            time,
            &document,
        )?;
        // The page links media by URL; only the ids are needed here, never the bytes.
        let media = document
            .media_ids()
            .into_iter()
            .map(|id| (id, String::new()))
            .collect();
        Ok(ArticleView {
            card,
            document,
            media,
        })
    }

    /// Raw media bytes for the public page; only media referenced by that revision is served.
    pub(crate) fn public_article_media(
        &self,
        token: &str,
        media: &str,
    ) -> Result<(String, Vec<u8>)> {
        let conn = self.conn()?;
        let (id, revision) = public_target(&conn, token)?;
        let head = head(&conn, &id)?;
        if head.status != "published" {
            return Err(fail(404, "文章已撤下"));
        }
        let json: String = conn.query_row(
            "SELECT document_json FROM social_content_revisions WHERE content_id=?1 AND revision=?2",
            params![id, revision],
            |r| r.get(0),
        )?;
        let document: ArticleDocument = serde_json::from_str(&json)?;
        if !document.media_ids().contains(media) {
            return Err(fail(404, "图片不存在"));
        }
        let row: Option<(String, Vec<u8>)> = conn
            .query_row(
                "SELECT mime_type,bytes FROM social_content_media WHERE id=?1 AND owner_id=?2",
                params![media, head.owner],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        row.ok_or_else(|| fail(404, "图片不存在"))
    }
}

fn public_target(conn: &Connection, token: &str) -> Result<(String, i64)> {
    if !valid_token(token) {
        return Err(fail(404, "分享链接无效"));
    }
    conn.query_row(
        "SELECT content_id,revision FROM social_content_distributions WHERE target_kind='public' AND target_id=?1",
        [token],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()?
    .ok_or_else(|| fail(404, "分享链接已失效"))
}
