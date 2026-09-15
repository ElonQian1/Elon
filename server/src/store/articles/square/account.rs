use super::*;
pub(super) fn read(conn: &Connection, owner: &str) -> Result<Account> {
    let row=conn.query_row("SELECT label,masked_key,generation,verified_at,length(encrypted_key)>0 FROM article_square_accounts WHERE owner_id=?1",[owner],|r|Ok(Account{label:r.get(0)?,masked_key:r.get(1)?,generation:r.get(2)?,verified_at:r.get(3)?,bound:r.get(4)?,creator_url:CREATOR_URL})).optional()?;
    Ok(row.unwrap_or(Account {
        bound: false,
        label: String::new(),
        masked_key: String::new(),
        generation: 0,
        verified_at: None,
        creator_url: CREATOR_URL,
    }))
}
impl Store {
    pub(crate) fn square_account(&self, owner: &str) -> Result<Account> {
        read(&*self.conn()?, owner)
    }
    pub(crate) fn square_bind(&self, owner: &str, label: &str, key: &str) -> Result<Account> {
        let key = key.trim();
        let label = label.trim();
        if key.len() < 16
            || key.len() > 512
            || !key.bytes().all(|c| c.is_ascii_graphic())
            || label.is_empty()
            || label.chars().count() > 60
        {
            return Err(fail(
                400,
                "请填写账号备注（最多60字）和有效的Square发帖凭证",
            ));
        }
        let encrypted = secret_envelope(owner, key)?;
        let fingerprint = digest(key);
        let masked = format!("{}…{}", &key[..5], &key[key.len() - 4..]);
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let current: Option<String> = tx
            .query_row(
                "SELECT key_fingerprint FROM article_square_accounts WHERE owner_id=?1",
                [owner],
                |r| r.get(0),
            )
            .optional()?;
        if current.as_deref() == Some(&fingerprint) {
            tx.execute("UPDATE article_square_accounts SET label=?2,encrypted_key=?3,masked_key=?4,updated_at=?5 WHERE owner_id=?1",params![owner,label,encrypted,masked,epoch()])?;
        } else {
            cancel_pending(&tx, owner, "发帖凭证已更换，请重新预览并创建任务")?;
            tx.execute("INSERT INTO article_square_accounts VALUES(?1,?2,?3,?4,?5,1,NULL,?6) ON CONFLICT(owner_id) DO UPDATE SET label=excluded.label,encrypted_key=excluded.encrypted_key,key_fingerprint=excluded.key_fingerprint,masked_key=excluded.masked_key,generation=article_square_accounts.generation+1,verified_at=NULL,updated_at=excluded.updated_at",params![owner,label,encrypted,fingerprint,masked,epoch()])?;
        }
        let result = read(&tx, owner)?;
        tx.commit()?;
        Ok(result)
    }
    pub(crate) fn square_unbind(&self, owner: &str) -> Result<Account> {
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        cancel_pending(&tx, owner, "账号已解绑，任务已取消")?;
        tx.execute("UPDATE article_square_accounts SET encrypted_key='',masked_key='',key_fingerprint='',generation=generation+1,verified_at=NULL,updated_at=?2 WHERE owner_id=?1",params![owner,epoch()])?;
        let result = read(&tx, owner)?;
        tx.commit()?;
        Ok(result)
    }
}
fn cancel_pending(conn: &Connection, owner: &str, message: &str) -> Result<()> {
    conn.execute("UPDATE article_square_jobs SET status='cancelled',message=?2,updated_at=?3 WHERE owner_id=?1 AND status IN ('queued','preparing','failed')",params![owner,message,epoch()])?;
    Ok(())
}
