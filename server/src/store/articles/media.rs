use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use sha2::{Digest, Sha256};
use std::io::Cursor;

#[derive(serde::Serialize)]
pub(crate) struct ArticleMedia {
    pub id: String,
    pub data_url: String,
}

impl Store {
    pub(crate) fn upload_article_media(&self, user: &str, encoded: &str) -> Result<ArticleMedia> {
        if encoded.len() > 710_000 {
            return Err(fail(413, "图片过大，请压缩后重试"));
        }
        let bytes = STANDARD
            .decode(encoded)
            .map_err(|_| fail(400, "图片编码无效"))?;
        if bytes.is_empty() || bytes.len() > 512 * 1024 {
            return Err(fail(413, "图片需小于512KB"));
        }
        let format = image::guess_format(&bytes).map_err(|_| fail(400, "图片格式无效"))?;
        let mime = match format {
            image::ImageFormat::Png => "image/png",
            image::ImageFormat::Jpeg => "image/jpeg",
            image::ImageFormat::WebP => "image/webp",
            _ => return Err(fail(400, "仅支持PNG、JPEG和WebP图片")),
        };
        let (width, height) = image::ImageReader::with_format(Cursor::new(&bytes), format)
            .into_dimensions()
            .map_err(|_| fail(400, "图片无法读取"))?;
        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err(fail(400, "图片尺寸需小于4096×4096"));
        }
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if let Some(id) = tx
            .query_row(
                "SELECT id FROM social_content_media WHERE owner_id=?1 AND sha256=?2",
                params![user, hash],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            return Ok(ArticleMedia {
                id,
                data_url: format!("data:{mime};base64,{}", STANDARD.encode(bytes)),
            });
        }
        let (count,total):(i64,i64)=tx.query_row("SELECT COUNT(*),COALESCE(SUM(length(bytes)),0) FROM social_content_media WHERE owner_id=?1",[user],|r|Ok((r.get(0)?,r.get(1)?)))?;
        if count >= 300 || total + bytes.len() as i64 > 100 * 1024 * 1024 {
            return Err(fail(400, "文章图片空间已达到当前上限"));
        }
        let image = image::load_from_memory_with_format(&bytes, format)
            .map_err(|_| fail(400, "图片内容无法解码"))?;
        let mut thumbnail = Cursor::new(Vec::new());
        image
            .thumbnail(400, 240)
            .to_rgb8()
            .write_to(&mut thumbnail, image::ImageFormat::Jpeg)?;
        let preview = format!(
            "data:image/jpeg;base64,{}",
            STANDARD.encode(thumbnail.into_inner())
        );
        let id = new_id("article_media");
        tx.execute(
            "INSERT INTO social_content_media VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![id, user, hash, mime, bytes, preview, now()],
        )?;
        tx.commit()?;
        Ok(ArticleMedia {
            id,
            data_url: format!("data:{mime};base64,{}", STANDARD.encode(bytes)),
        })
    }
}
