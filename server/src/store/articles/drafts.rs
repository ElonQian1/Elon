use super::*;

impl Store {
    pub(crate) fn create_article(
        &self,
        user: &str,
        document: ArticleDocument,
    ) -> Result<ArticleView> {
        let conn = self.conn()?;
        document.validate(&conn, user, false)?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM social_contents WHERE owner_id=?1 AND status!='withdrawn'",
            [user],
            |r| r.get(0),
        )?;
        if count >= 1000 {
            return Err(fail(400, "文章数量已达到当前上限"));
        }
        let id = new_id("article");
        let time = now();
        conn.execute(
            "INSERT INTO social_contents VALUES (?1,?2,'article',?3,1,'draft',?4,?4)",
            params![id, user, serde_json::to_string(&document)?, time],
        )?;
        reads::draft_view(&conn, user, &id)
    }
    pub(crate) fn article_draft(&self, user: &str, id: &str) -> Result<ArticleView> {
        let conn = self.conn()?;
        reads::draft_view(&conn, user, id)
    }
    pub(crate) fn save_article(
        &self,
        user: &str,
        id: &str,
        version: i64,
        document: ArticleDocument,
    ) -> Result<ArticleView> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let prior = owned(&tx, user, id)?;
        if prior.status == "withdrawn" {
            return Err(fail(409, "文章已撤下，不能再修改"));
        }
        if prior.version != version {
            return Err(fail(409, "草稿已在其他设备修改，请保留当前内容并重新读取"));
        }
        document.validate(&tx, user, false)?;
        connless_save(&tx, id, document)?;
        let view = reads::draft_view(&tx, user, id)?;
        tx.commit()?;
        Ok(view)
    }
    pub(crate) fn withdraw_article(&self, user: &str, id: &str, version: i64) -> Result<()> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let value = owned(&tx, user, id)?;
        if value.status == "withdrawn" {
            return Ok(());
        }
        if value.version != version {
            return Err(fail(409, "文章已发生变化，请重新读取后操作"));
        }
        tx.execute(
            "UPDATE social_contents SET status='withdrawn',updated_at=?2 WHERE id=?1",
            params![id, now()],
        )?;
        tx.commit()?;
        Ok(())
    }
}
fn connless_save(conn: &Connection, id: &str, document: ArticleDocument) -> Result<()> {
    conn.execute("UPDATE social_contents SET draft_json=?2,edit_version=edit_version+1,updated_at=?3 WHERE id=?1",params![id,serde_json::to_string(&document)?,now()])?;
    Ok(())
}
