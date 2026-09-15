use super::*;
pub(super) fn image_bytes(conn: &Connection, owner: &str, id: &str) -> Result<(String, Vec<u8>)> {
    conn.query_row(
        "SELECT mime_type,bytes FROM social_content_media WHERE id=?1 AND owner_id=?2",
        params![id, owner],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()?
    .ok_or_else(|| fail(400, "图片不存在或不属于当前作者"))
}
pub(super) fn video_metadata(conn: &Connection, owner: &str, id: &str) -> Result<(f64, String)> {
    conn.query_row(
        "SELECT duration,cover_id FROM article_square_videos WHERE id=?1 AND owner_id=?2",
        params![id, owner],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .optional()?
    .ok_or_else(|| fail(400, "视频不存在或不属于当前作者"))
}
impl Store {
    pub(crate) fn square_video(
        &self,
        owner: &str,
        bytes: Vec<u8>,
        duration: f64,
        cover: &str,
    ) -> Result<Value> {
        if bytes.len() < 12
            || bytes.len() > 32 * 1024 * 1024
            || !duration.is_finite()
            || duration <= 0.0
            || duration > 600.0
        {
            return Err(fail(400, "视频需小于32MB、时长不超过10分钟"));
        }
        let mime = if bytes.get(4..8) == Some(b"ftyp") {
            "video/mp4"
        } else if bytes.starts_with(&[0x1a, 0x45, 0xdf, 0xa3]) {
            "video/webm"
        } else {
            return Err(fail(400, "请选择MP4或WebM视频"));
        };
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        image_bytes(&tx, owner, cover)?;
        let prior: Option<String> = tx
            .query_row(
                "SELECT id FROM article_square_videos WHERE owner_id=?1 AND sha256=?2",
                params![owner, hash],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = prior {
            return Ok(json!({"id":id}));
        }
        // Expire unreferenced staging uploads; durable job snapshots retain their media.
        tx.execute("DELETE FROM article_square_videos WHERE owner_id=?1 AND created_at<?2 AND NOT EXISTS(SELECT 1 FROM article_square_jobs j WHERE json_extract(j.payload_json,'$.selection.video_id')=article_square_videos.id)",params![owner,epoch()-86400])?;
        let total: i64 = tx.query_row(
            "SELECT COALESCE(SUM(length(bytes)),0) FROM article_square_videos WHERE owner_id=?1",
            [owner],
            |r| r.get(0),
        )?;
        if total + bytes.len() as i64 > 256 * 1024 * 1024 {
            return Err(fail(413, "视频素材空间已达到256MB上限"));
        }
        let id = new_id("square_video");
        tx.execute(
            "INSERT INTO article_square_videos VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            params![id, owner, bytes, mime, duration, cover, hash, epoch()],
        )?;
        tx.commit()?;
        Ok(json!({"id":id}))
    }
    pub(super) fn square_media_bytes(
        &self,
        owner: &str,
        id: &str,
        video: bool,
    ) -> Result<(String, Vec<u8>)> {
        let conn = self.conn()?;
        if !video {
            return image_bytes(&conn, owner, id);
        }
        conn.query_row(
            "SELECT mime_type,bytes FROM article_square_videos WHERE id=?1 AND owner_id=?2",
            params![id, owner],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| fail(400, "视频不可读取"))
    }
    pub(super) fn square_video_metadata(&self, owner: &str, id: &str) -> Result<(f64, String)> {
        video_metadata(&*self.conn()?, owner, id)
    }
}
