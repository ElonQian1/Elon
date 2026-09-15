use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::io::Cursor;
const MAX_IMAGE_BYTES: usize = 12 * 1024 * 1024;

fn clean_image(encoded: &str) -> Result<Vec<u8>> {
    if encoded.len() > 16 * 1024 * 1024 {
        return Err(fail(413, "Image exceeds 12 MiB"));
    }
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| fail(400, "Invalid image base64"))?;
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err(fail(413, "Image exceeds 12 MiB"));
    }
    let format = image::guess_format(&bytes).map_err(|_| fail(400, "Invalid image"))?;
    if !matches!(
        format,
        image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::WebP
    ) {
        return Err(fail(400, "Only PNG, JPEG and WebP images are supported"));
    }
    let (width, height) = image::ImageReader::with_format(Cursor::new(&bytes), format)
        .into_dimensions()
        .map_err(|_| fail(400, "Invalid image dimensions"))?;
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err(fail(400, "Image must be at most 4096 by 4096 pixels"));
    }
    // Re-encoding pixels drops EXIF, comments, source URLs and appended provider bytes.
    let decoded = image::load_from_memory_with_format(&bytes, format)
        .map_err(|_| fail(400, "Image decode failed"))?;
    let mut clean = Cursor::new(Vec::new());
    decoded
        .write_to(&mut clean, image::ImageFormat::Png)
        .map_err(|_| fail(400, "Image encoding failed"))?;
    let clean = clean.into_inner();
    if clean.len() > MAX_IMAGE_BYTES {
        return Err(fail(413, "Sanitized image exceeds 12 MiB"));
    }
    Ok(clean)
}

impl Store {
    pub(crate) fn upload_ai_snapshot_asset(
        &self,
        user: &str,
        group: &str,
        encoded: &str,
    ) -> Result<SnapshotAsset> {
        {
            let conn = self.conn()?;
            member(&conn, user, group)?;
        }
        // Opportunistic cleanup has its own transaction; an unknown reference fails closed.
        if self.cleanup_ai_snapshot_orphans(user).is_err() {
            tracing::warn!("Snapshot orphan cleanup deferred; assets retained");
        }
        let clean = clean_image(encoded)?;
        // The existing uploader owns deduplication, owner quota, thumbnails and blob storage.
        let size_bytes = clean.len();
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        member(&tx, user, group)?;
        let (media, created) = Self::persist_social_image(&tx, user, clean, MAX_IMAGE_BYTES)?;
        let uploaded_at = chrono::Utc::now().timestamp();
        if created {
            tx.execute("INSERT INTO social_snapshot_orphan_media(media_id,owner_id,created_at) VALUES (?1,?2,?3)",
                params![media.id,user,uploaded_at])?;
        }
        tx.execute(
            "INSERT INTO social_snapshot_asset_grants(owner_id,group_id,media_id,uploaded_at) VALUES (?1,?2,?3,?4)
             ON CONFLICT(owner_id,group_id,media_id) DO UPDATE SET uploaded_at=excluded.uploaded_at",
            params![user, group, media.id, uploaded_at],
        )?;
        tx.commit()?;
        Ok(SnapshotAsset {
            asset_id: media.id,
            mime_type: "image/png".into(),
            size_bytes,
        })
    }

    pub(crate) fn read_ai_snapshot_asset(
        &self,
        user: &str,
        group: &str,
        snapshot: &str,
        asset: &str,
    ) -> Result<(String, Vec<u8>)> {
        let conn = self.conn()?;
        let view = readable(&conn, user, group, snapshot)?;
        if !view.document.asset_ids().contains(asset) {
            return Err(fail(404, "Asset unavailable"));
        }
        conn.query_row(
            "SELECT m.mime_type,m.bytes FROM social_content_media m
            JOIN social_snapshot_asset_grants g ON g.media_id=m.id AND g.owner_id=m.owner_id
            WHERE m.id=?1 AND m.owner_id=?2 AND g.group_id=?3",
            params![asset, view.owner_id, group],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| fail(404, "Asset unavailable"))
    }
}
