use super::*;
use std::collections::BTreeSet;

pub(crate) const CARD_PREFIX: &str = "【一龙文章】\n";

impl Store {
    pub(crate) fn publish_article(
        &self,
        user: &str,
        id: &str,
        version: i64,
        groups: &[String],
    ) -> Result<ArticlePublishResult> {
        let groups: BTreeSet<&str> = groups.iter().map(|g| g.trim()).collect();
        if groups.is_empty() || groups.len() > 10 || groups.contains("") {
            return Err(fail(400, "请选择1至10个群聊"));
        }
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let head = owned(&tx, user, id)?;
        if head.status == "withdrawn" {
            return Err(fail(409, "文章已撤下，不能重新发布"));
        }
        if head.version != version {
            return Err(fail(409, "草稿版本已变化，请重新预览后发布"));
        }
        head.document.validate(&tx, user, true)?;
        for group in &groups {
            member(&tx, user, group)?;
        }
        let time = now();
        tx.execute(
            "INSERT OR IGNORE INTO social_content_revisions VALUES (?1,?2,?3,?4)",
            params![id, version, serde_json::to_string(&head.document)?, time],
        )?;
        let content = format!(
            "{CARD_PREFIX}{}",
            serde_json::json!({"schema":1,"article_id":id,"revision":version,"title":head.document.title,"summary":head.document.summary})
        );
        let mut messages = vec![];
        let mut already_shared_groups = vec![];
        for group in groups {
            let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM social_content_distributions WHERE content_id=?1 AND revision=?2 AND target_kind='group' AND target_id=?3)",params![id,version,group],|r|r.get(0))?;
            if exists {
                already_shared_groups.push(group.into());
                continue;
            }
            let message =
                super::super::groups::send::insert_message(&tx, user, group, &content, None)?;
            tx.execute(
                "INSERT INTO social_content_distributions VALUES (?1,?2,'group',?3,?4,?5)",
                params![id, version, group, message.id, time],
            )?;
            messages.push(message);
        }
        tx.execute(
            "UPDATE social_contents SET status='published',updated_at=?2 WHERE id=?1",
            params![id, time],
        )?;
        tx.commit()?;
        Ok(ArticlePublishResult {
            article_id: id.into(),
            revision: version,
            messages,
            already_shared_groups,
        })
    }
}
