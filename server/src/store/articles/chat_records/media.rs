use super::*;

fn media_type(bytes: &[u8]) -> Result<&'static str> {
    if bytes.is_empty() || bytes.len() > MAX_ASSET {
        return Err(fail(413, "单个附件不能超过 12 MiB"));
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Ok("image/jpeg");
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok("image/png");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Ok("image/gif");
    }
    if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        return Ok("image/webp");
    }
    if bytes.get(4..8) == Some(b"ftyp") {
        return Ok("video/mp4");
    }
    if bytes.starts_with(b"%PDF-") {
        return Ok("application/pdf");
    }
    // Unknown formats remain downloadable files, never executable inline documents.
    Ok("application/octet-stream")
}

impl Store {
    pub(crate) fn upload_chat_record_asset(
        &self,
        user: &str,
        group: &str,
        bytes: &[u8],
    ) -> Result<Asset> {
        let mime = media_type(bytes)?;
        if mime.starts_with("image/") {
            let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
                .with_guessed_format()
                .map_err(|_| fail(400, "图片格式无效"))?;
            let (w, h) = reader
                .into_dimensions()
                .map_err(|_| fail(400, "图片格式无效"))?;
            if w == 0 || h == 0 || w > 16384 || h > 16384 || w as u64 * h as u64 > 40_000_000 {
                return Err(fail(413, "图片尺寸过大"));
            }
        }
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        member(&tx, user, group)?;
        tx.execute("DELETE FROM social_chat_record_assets WHERE owner_id=?1 AND created_at<?2
            AND NOT EXISTS(SELECT 1 FROM social_chat_record_asset_refs r WHERE r.asset_id=social_chat_record_assets.id)",
            params![user, chrono::Utc::now().timestamp() - 86400])?;
        let hash = format!("{:x}", Sha256::digest(bytes));
        let prior: Option<String> = tx.query_row("SELECT id FROM social_chat_record_assets WHERE owner_id=?1 AND group_id=?2 AND sha256=?3",
            params![user,group,hash], |r| r.get(0)).optional()?;
        let id = if let Some(id) = prior {
            id
        } else {
            let total: i64 = tx.query_row("SELECT COALESCE(SUM(length(bytes)),0) FROM social_chat_record_assets WHERE owner_id=?1", [user], |r| r.get(0))?;
            if total + bytes.len() as i64 > 256 * 1024 * 1024 {
                return Err(fail(413, "附件存储额度已满"));
            }
            let id = new_id("record_asset");
            tx.execute(
                "INSERT INTO social_chat_record_assets VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    id,
                    user,
                    group,
                    hash,
                    mime,
                    bytes,
                    chrono::Utc::now().timestamp()
                ],
            )?;
            id
        };
        tx.commit()?;
        Ok(Asset {
            asset_id: id,
            mime_type: mime.into(),
            size_bytes: bytes.len(),
        })
    }

    pub(crate) fn read_chat_record_asset(
        &self,
        user: &str,
        group: &str,
        record: &str,
        asset: &str,
    ) -> Result<(String, Vec<u8>)> {
        let conn = self.conn()?;
        let view = readable(&conn, user, group, record)?;
        if !view.document.asset_ids().contains(asset) {
            return Err(fail(404, "附件不可访问"));
        }
        conn.query_row("SELECT mime_type,bytes FROM social_chat_record_assets WHERE id=?1 AND group_id=?2 AND owner_id=?3",
            params![asset,group,view.owner_id], |r| Ok((r.get(0)?,r.get(1)?))).optional()?.ok_or_else(|| fail(404, "附件不可访问"))
    }
}
