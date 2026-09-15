use super::*;
impl Store {
    pub(crate) fn square_cancel(&self, owner: &str, id: &str) -> Result<Job> {
        let conn = self.conn()?;
        let job = queue::read(&conn, owner, id)?;
        if job.status == "cancelled" {
            return Ok(job);
        }
        if !matches!(job.status.as_str(), "queued" | "failed" | "preparing") {
            return Err(fail(
                409,
                "已进入提交阶段或已完成，不能取消；请到币安管理已发布内容",
            ));
        }
        conn.execute("UPDATE article_square_jobs SET status='cancelled',message='已取消',updated_at=?3 WHERE id=?1 AND owner_id=?2 AND status IN ('queued','failed','preparing')",params![id,owner,epoch()])?;
        queue::read(&conn, owner, id)
    }
    pub(crate) fn square_retry(&self, owner: &str, id: &str) -> Result<Job> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let job = queue::read(&tx, owner, id)?;
        if matches!(job.status.as_str(), "queued" | "preparing" | "submitting") {
            return Ok(job);
        }
        if !matches!(job.status.as_str(), "failed" | "cancelled") {
            return Err(fail(409, "这条任务不能重试；结果待核实的任务需先核实"));
        }
        let h = owned(&tx, owner, &job.article_id)?;
        if h.status == "withdrawn" {
            return Err(fail(409, "文章已撤下，不能重试"));
        }
        let account = account::read(&tx, owner)?;
        let generation: i64 = tx.query_row(
            "SELECT generation FROM article_square_jobs WHERE id=?1",
            [id],
            |r| r.get(0),
        )?;
        if !account.bound || generation != account.generation {
            return Err(fail(409, "凭证已变化，请从文章重新预览发布"));
        }
        let pending: i64 = tx.query_row("SELECT COUNT(*) FROM article_square_jobs WHERE owner_id=?1 AND status IN ('queued','preparing','submitting')", [owner], |r|r.get(0))?;
        if pending >= 100 {
            return Err(fail(429, "待发布任务已达100条，请等待或取消部分任务"));
        }
        tx.execute("UPDATE article_square_jobs SET status='queued',scheduled_at=?2,updated_at=?2,message='等待重新发布' WHERE id=?1",params![id,epoch()])?;
        let job = queue::read(&tx, owner, id)?;
        tx.commit()?;
        Ok(job)
    }
    pub(crate) fn square_resolve(
        &self,
        owner: &str,
        id: &str,
        post_id: Option<&str>,
        not_published: bool,
    ) -> Result<Job> {
        if not_published == post_id.is_some() || post_id.is_some_and(|id| post_link(id).is_none()) {
            return Err(fail(400, "请填写有效帖子ID，或明确确认未发布"));
        }
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let job = queue::read(&tx, owner, id)?;
        if job.status != "uncertain" {
            return Err(fail(409, "仅结果待核实的任务需要人工处理"));
        }
        let (status, message) = if not_published {
            ("failed", "作者已核实未发布，可以显式重试")
        } else {
            ("published", "作者已在币安核实并补充链接")
        };
        tx.execute("UPDATE article_square_jobs SET status=?3,post_id=?4,message=?5,updated_at=?6 WHERE id=?1 AND owner_id=?2 AND status='uncertain'",params![id,owner,status,post_id,message,epoch()])?;
        let job = queue::read(&tx, owner, id)?;
        tx.commit()?;
        Ok(job)
    }
    pub(super) fn square_work_key(&self, work: &Work) -> Result<String> {
        let conn = self.conn()?;
        work_key(&conn, work)
    }
    pub(super) fn square_reserve_upload(&self, work: &Work) -> Result<()> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        work_key(&tx, work)?;
        let n: i64 = tx.query_row(
            "SELECT COUNT(*) FROM article_square_uploads WHERE owner_id=?1 AND created_at>?2",
            params![work.owner, epoch() - 86400],
            |r| r.get(0),
        )?;
        if n >= 400 {
            return Err(fail(429, "最近24小时媒体上传已达到400次，请稍后重试"));
        }
        tx.execute(
            "INSERT INTO article_square_uploads VALUES(?1,?2,?3)",
            params![new_id("square_upload"), work.owner, epoch()],
        )?;
        tx.execute(
            "DELETE FROM article_square_uploads WHERE created_at<?1",
            [epoch() - 172800],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub(super) fn square_submitting(&self, work: &Work) -> Result<()> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        work_key(&tx, work)?;
        let n: i64 = tx.query_row(
            "SELECT COUNT(*) FROM article_square_submissions WHERE owner_id=?1 AND created_at>?2",
            params![work.owner, epoch() - 86400],
            |r| r.get(0),
        )?;
        if n >= 100 {
            return Err(fail(429, "最近24小时发帖尝试已达到100次，请稍后重试"));
        }
        tx.execute(
            "INSERT INTO article_square_submissions VALUES(?1,?2,?3)",
            params![new_id("square_submit"), work.owner, epoch()],
        )?;
        tx.execute(
            "DELETE FROM article_square_submissions WHERE created_at<?1",
            [epoch() - 172800],
        )?;
        let n=tx.execute("UPDATE article_square_jobs SET status='submitting',submitted_at=?2,updated_at=?2,message='已提交，请等待回执' WHERE id=?1 AND status='preparing'",params![work.id,epoch()])?;
        if n != 1 {
            return Err(fail(409, "任务已取消或状态变化"));
        }
        tx.commit()?;
        Ok(())
    }
    pub(super) fn square_finish(
        &self,
        work: &Work,
        status: &str,
        message: &str,
        post_id: Option<&str>,
    ) -> Result<()> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let n=tx.execute("UPDATE article_square_jobs SET status=?2,message=?3,post_id=?4,updated_at=?5 WHERE id=?1 AND status IN ('preparing','submitting')",params![work.id,status,message,post_id,epoch()])?;
        if n > 0 && status == "published" {
            tx.execute("UPDATE article_square_accounts SET verified_at=?3 WHERE owner_id=?1 AND generation=?2",params![work.owner,work.generation,epoch()])?;
        }
        tx.commit()?;
        Ok(())
    }
}
fn work_key(conn: &Connection, work: &Work) -> Result<String> {
    let encrypted:Option<String>=conn.query_row("SELECT a.encrypted_key FROM article_square_accounts a JOIN article_square_jobs j ON j.owner_id=a.owner_id JOIN social_contents c ON c.id=j.content_id WHERE a.owner_id=?1 AND a.generation=?2 AND j.id=?3 AND j.status='preparing' AND c.status!='withdrawn' AND length(a.encrypted_key)>0",params![work.owner,work.generation,work.id],|r|r.get(0)).optional()?;
    reveal(
        &work.owner,
        &encrypted.ok_or_else(|| fail(409, "任务已取消、文章已撤下或账号已更换"))?,
    )
}
