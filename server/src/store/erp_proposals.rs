use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, Row};
use std::collections::BTreeSet;

use crate::erp_blueprint::model::{
    ErpFeatureProposal, ErpFeatureSignal, SubmitFeatureSignalRequest,
};

use super::{new_id, now, Store};

impl Store {
    pub(crate) fn upsert_erp_feature_signal(
        &self,
        blueprint_id: &str,
        instance_id: &str,
        need_key: &str,
        request: &SubmitFeatureSignalRequest,
        actor_user_id: &str,
    ) -> Result<ErpFeatureSignal> {
        let id = new_id("erp_signal");
        let timestamp = now();
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO erp_feature_signals (
               id, blueprint_id, instance_id, need_key, requirement_summary, industry,
               requested_outcome, evidence_json, classification, created_by, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
             ON CONFLICT(instance_id, need_key) DO UPDATE SET
               requirement_summary=excluded.requirement_summary,
               industry=excluded.industry,
               requested_outcome=excluded.requested_outcome,
               evidence_json=excluded.evidence_json,
               classification=excluded.classification,
               created_by=excluded.created_by,
               updated_at=excluded.updated_at",
            params![
                id,
                blueprint_id.trim(),
                instance_id.trim(),
                need_key.trim(),
                request.requirement_summary.trim(),
                request.industry.trim(),
                request.requested_outcome.trim(),
                serde_json::to_string(&request.evidence)?,
                request.classification.trim(),
                actor_user_id.trim(),
                timestamp,
            ],
        )?;
        refresh_erp_feature_proposal(&tx, blueprint_id, need_key)?;
        tx.commit()?;
        drop(conn);
        self.erp_feature_signal(instance_id, need_key)
    }

    pub(crate) fn erp_feature_signal(
        &self,
        instance_id: &str,
        need_key: &str,
    ) -> Result<ErpFeatureSignal> {
        self.conn()?
            .query_row(
                &format!("{SIGNAL_SELECT} WHERE instance_id=?1 AND need_key=?2"),
                params![instance_id.trim(), need_key.trim()],
                signal_from_row,
            )
            .map_err(|error| anyhow!(error).context("ERP 需求信号不存在"))
    }

    pub(crate) fn list_erp_feature_proposals(
        &self,
        blueprint_id: &str,
    ) -> Result<Vec<ErpFeatureProposal>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(&format!(
            "{PROPOSAL_SELECT} WHERE blueprint_id=?1 ORDER BY support_count DESC, updated_at DESC"
        ))?;
        let result = stmt
            .query_map(params![blueprint_id.trim()], proposal_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into);
        result
    }

    pub(crate) fn erp_feature_proposal(&self, proposal_id: &str) -> Result<ErpFeatureProposal> {
        self.conn()?
            .query_row(
                &format!("{PROPOSAL_SELECT} WHERE id=?1"),
                params![proposal_id.trim()],
                proposal_from_row,
            )
            .map_err(|error| anyhow!(error).context("ERP 通用功能提案不存在"))
    }

    pub(crate) fn decide_erp_feature_proposal(
        &self,
        proposal_id: &str,
        decision: &str,
        note: &str,
        actor_user_id: &str,
    ) -> Result<ErpFeatureProposal> {
        if !matches!(decision, "accepted" | "rejected") {
            bail!("提案决策只能是 accepted 或 rejected");
        }
        let current = self.erp_feature_proposal(proposal_id)?;
        if current.status != "candidate" {
            bail!("只有 candidate 状态的提案可以决策");
        }
        self.conn()?.execute(
            "UPDATE erp_feature_proposals SET status=?1, decision_by=?2, decision_note=?3, updated_at=?4 WHERE id=?5",
            params![decision, actor_user_id.trim(), note.trim(), now(), proposal_id.trim()],
        )?;
        self.erp_feature_proposal(proposal_id)
    }
}

fn refresh_erp_feature_proposal(
    conn: &Connection,
    blueprint_id: &str,
    need_key: &str,
) -> Result<()> {
    let (support_count, title, summary): (i64, String, String) = conn.query_row(
        "SELECT COUNT(*), MIN(requirement_summary), MIN(requirement_summary)
             FROM erp_feature_signals WHERE blueprint_id=?1 AND need_key=?2",
        params![blueprint_id.trim(), need_key.trim()],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    let mut stmt = conn.prepare(
        "SELECT DISTINCT industry FROM erp_feature_signals
             WHERE blueprint_id=?1 AND need_key=?2 ORDER BY industry",
    )?;
    let industries = stmt
        .query_map(params![blueprint_id.trim(), need_key.trim()], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<rusqlite::Result<BTreeSet<_>>>()?
        .into_iter()
        .collect::<Vec<_>>();
    drop(stmt);
    let id = new_id("erp_proposal");
    let timestamp = now();
    conn.execute(
        "INSERT INTO erp_feature_proposals (
               id, blueprint_id, need_key, title, summary, status, support_count,
               industries_json, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, 'candidate', ?6, ?7, ?8, ?8)
             ON CONFLICT(blueprint_id, need_key) DO UPDATE SET
               title=excluded.title, summary=excluded.summary,
               support_count=excluded.support_count,
               industries_json=excluded.industries_json, updated_at=excluded.updated_at",
        params![
            id,
            blueprint_id.trim(),
            need_key.trim(),
            title,
            summary,
            support_count,
            serde_json::to_string(&industries)?,
            timestamp,
        ],
    )?;
    Ok(())
}

const SIGNAL_SELECT: &str = "SELECT id, blueprint_id, instance_id, need_key,
 requirement_summary, industry, requested_outcome, evidence_json, classification,
 created_by, created_at, updated_at FROM erp_feature_signals";
const PROPOSAL_SELECT: &str = "SELECT id, blueprint_id, need_key, title, summary,
 status, support_count, industries_json, matter_id, decision_by, decision_note,
 created_at, updated_at FROM erp_feature_proposals";

fn signal_from_row(row: &Row<'_>) -> rusqlite::Result<ErpFeatureSignal> {
    Ok(ErpFeatureSignal {
        id: row.get(0)?,
        blueprint_id: row.get(1)?,
        instance_id: row.get(2)?,
        need_key: row.get(3)?,
        requirement_summary: row.get(4)?,
        industry: row.get(5)?,
        requested_outcome: row.get(6)?,
        evidence: decode(row, 7)?,
        classification: row.get(8)?,
        created_by: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn proposal_from_row(row: &Row<'_>) -> rusqlite::Result<ErpFeatureProposal> {
    Ok(ErpFeatureProposal {
        id: row.get(0)?,
        blueprint_id: row.get(1)?,
        need_key: row.get(2)?,
        title: row.get(3)?,
        summary: row.get(4)?,
        status: row.get(5)?,
        support_count: row.get(6)?,
        industries: decode(row, 7)?,
        matter_id: row.get(8)?,
        decision_by: row.get(9)?,
        decision_note: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn decode<T: serde::de::DeserializeOwned>(row: &Row<'_>, index: usize) -> rusqlite::Result<T> {
    let raw: String = row.get(index)?;
    serde_json::from_str(&raw).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            raw.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}
