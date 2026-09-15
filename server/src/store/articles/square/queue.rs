use super::*;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Enqueue {
    pub selection: Selection,
    pub preview_hash: String,
    pub request_key: String,
    pub public_confirmed: bool,
    pub conversion_confirmed: bool,
    #[serde(default)]
    pub scheduled_at: Option<i64>,
}
const COLUMNS:&str="id,content_id,revision,payload_json,status,scheduled_at,created_at,updated_at,attempts,message,post_id";
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Job> {
    let raw: String = r.get(3)?;
    let payload: Payload = serde_json::from_str(&raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e))
    })?;
    let id: Option<String> = r.get(10)?;
    Ok(Job {
        id: r.get(0)?,
        article_id: r.get(1)?,
        revision: r.get(2)?,
        title: payload.title,
        mode: payload.selection.mode,
        status: r.get(4)?,
        scheduled_at: r.get(5)?,
        created_at: r.get(6)?,
        updated_at: r.get(7)?,
        attempts: r.get(8)?,
        message: r.get(9)?,
        post_url: id.as_deref().and_then(post_link),
    })
}
pub(super) fn read(conn: &Connection, owner: &str, id: &str) -> Result<Job> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM article_square_jobs WHERE id=?1 AND owner_id=?2"),
        params![id, owner],
        row,
    )
    .optional()?
    .ok_or_else(|| fail(404, "发布记录不存在"))
}
impl Store {
    pub(crate) fn square_job(&self, owner: &str, id: &str) -> Result<Job> {
        read(&*self.conn()?, owner, id)
    }
    pub(crate) fn square_jobs(&self, owner: &str, offset: i64) -> Result<Value> {
        if !(0..=100_000).contains(&offset) {
            return Err(fail(400, "分页参数无效"));
        }
        let conn = self.conn()?;
        let mut stmt=conn.prepare(&format!("SELECT {COLUMNS} FROM article_square_jobs WHERE owner_id=?1 ORDER BY created_at DESC,id DESC LIMIT 21 OFFSET ?2"))?;
        let mut items = stmt
            .query_map(params![owner, offset], row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let next = if items.len() > 20 {
            items.pop();
            Some(offset + 20)
        } else {
            None
        };
        Ok(json!({"items":items,"next_offset":next}))
    }
    pub(crate) fn square_enqueue(&self, owner: &str, input: Enqueue) -> Result<Job> {
        if !input.public_confirmed || !input.conversion_confirmed {
            return Err(fail(400, "请预览并确认公开发布及内容转换"));
        }
        if input.request_key.len() < 16
            || input.request_key.len() > 96
            || !input
                .request_key
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return Err(fail(400, "发布请求标识无效"));
        }
        let now = epoch();
        let at = input.scheduled_at.unwrap_or(now);
        let request_hash = digest(&json!({"selection":input.selection,"preview_hash":input.preview_hash,"scheduled_at":input.scheduled_at}).to_string());
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        // A lost response remains recoverable even after draft edits, expiry of
        // the original schedule, cancellation, or credential rotation.
        let previous: Option<(String,String)> = tx.query_row("SELECT job_id,request_hash FROM article_square_requests WHERE owner_id=?1 AND request_key=?2",params![owner,input.request_key],|r| Ok((r.get(0)?,r.get(1)?))).optional()?;
        if let Some((id, hash)) = previous {
            if hash != request_hash {
                return Err(fail(409, "请求标识已用于其他内容或发送时间"));
            }
            return read(&tx, owner, &id);
        }
        if at > now + 30 * 86400 || at < now - 60 {
            return Err(fail(400, "定时时间需在未来30天内"));
        }
        let account = account::read(&tx, owner)?;
        if !account.bound {
            return Err(fail(409, "请先绑定币安广场账号"));
        }
        let payload = conversion(&tx, owner, &input.selection)?;
        if preview_hash(&payload, account.generation)? != input.preview_hash {
            return Err(fail(409, "预览已变化，请重新确认"));
        }
        let encoded = serde_json::to_string(&payload)?;
        let hash = digest(&encoded);
        let fingerprint: String = tx.query_row(
            "SELECT key_fingerprint FROM article_square_accounts WHERE owner_id=?1",
            [owner],
            |r| r.get(0),
        )?;
        let prior:Option<String>=tx.query_row("SELECT id FROM article_square_jobs WHERE owner_id=?1 AND key_fingerprint=?2 AND payload_hash=?3",params![owner,fingerprint,hash],|r|r.get(0)).optional()?;
        if let Some(id) = prior {
            // Re-binding the same key can resume an explicitly previewed,
            // cancelled job; completed/uncertain jobs are never resubmitted.
            let resuming: bool = tx.query_row(
                "SELECT status='cancelled' AND generation!=?2 FROM article_square_jobs WHERE id=?1",
                params![id, account.generation],
                |r| r.get(0),
            )?;
            if resuming {
                let pending: i64 = tx.query_row("SELECT COUNT(*) FROM article_square_jobs WHERE owner_id=?1 AND status IN ('queued','preparing','submitting')", [owner], |r| r.get(0))?;
                if pending >= 100 {
                    return Err(fail(429, "待发布任务已达100条，请等待或取消部分任务"));
                }
            }
            tx.execute("UPDATE article_square_jobs SET generation=?3,status='queued',scheduled_at=?4,updated_at=?5,message='已重新确认，等待发布' WHERE id=?1 AND owner_id=?2 AND status='cancelled' AND generation!=?3",params![id,owner,account.generation,at,now])?;
            tx.execute(
                "INSERT INTO article_square_requests VALUES(?1,?2,?3,?4)",
                params![owner, input.request_key, request_hash, id],
            )?;
            let job = read(&tx, owner, &id)?;
            tx.commit()?;
            return Ok(job);
        }
        let pending:i64=tx.query_row("SELECT COUNT(*) FROM article_square_jobs WHERE owner_id=?1 AND status IN ('queued','preparing','submitting')",[owner],|r|r.get(0))?;
        if pending >= 100 {
            return Err(fail(429, "待发布任务已达100条，请等待或取消部分任务"));
        }
        let h = owned(&tx, owner, &input.selection.article_id)?;
        tx.execute(
            "INSERT OR IGNORE INTO social_content_revisions VALUES(?1,?2,?3,?4)",
            params![
                input.selection.article_id,
                input.selection.version,
                serde_json::to_string(&h.document)?,
                super::super::now()
            ],
        )?;
        let id = new_id("square_job");
        tx.execute("INSERT INTO article_square_jobs(id,owner_id,content_id,revision,generation,key_fingerprint,request_key,payload_hash,payload_json,status,scheduled_at,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,'queued',?10,?11,?11)",params![id,owner,input.selection.article_id,input.selection.version,account.generation,fingerprint,input.request_key,hash,encoded,at,now])?;
        tx.execute(
            "INSERT INTO article_square_requests VALUES(?1,?2,?3,?4)",
            params![owner, input.request_key, request_hash, id],
        )?;
        let job = read(&tx, owner, &id)?;
        tx.commit()?;
        Ok(job)
    }
    pub(super) fn square_claim(&self) -> Result<Option<Work>> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let now = epoch();
        tx.execute("UPDATE article_square_jobs SET status='failed',message='准备任务中断，尚未提交，可重试',updated_at=?1 WHERE status='preparing' AND updated_at<?2",params![now,now-900])?;
        tx.execute("UPDATE article_square_jobs SET status='uncertain',message='提交过程未取得完整回执，请到币安核实后处理',updated_at=?1 WHERE status='submitting' AND updated_at<?2",params![now,now-180])?;
        tx.execute("UPDATE article_square_jobs SET status='cancelled',message='文章已撤下或凭证已更换',updated_at=?1 WHERE status IN ('queued','preparing') AND (EXISTS(SELECT 1 FROM social_contents c WHERE c.id=content_id AND c.status='withdrawn') OR NOT EXISTS(SELECT 1 FROM article_square_accounts a WHERE a.owner_id=article_square_jobs.owner_id AND a.generation=article_square_jobs.generation AND length(a.encrypted_key)>0))",[now])?;
        let row:Option<(String,String,i64,String)>=tx.query_row("SELECT id,owner_id,generation,payload_json FROM article_square_jobs WHERE status='queued' AND scheduled_at<=?1 ORDER BY scheduled_at,created_at LIMIT 1",[now],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional()?;
        let Some((id, owner, generation, raw)) = row else {
            tx.commit()?;
            return Ok(None);
        };
        tx.execute("UPDATE article_square_jobs SET status='preparing',attempts=attempts+1,updated_at=?2,message='正在准备媒体' WHERE id=?1",params![id,now])?;
        let work = Work {
            id,
            owner,
            generation,
            payload: serde_json::from_str(&raw)?,
        };
        tx.commit()?;
        Ok(Some(work))
    }
}
