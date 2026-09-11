//! Authenticated source snapshots for a separate treasury executor; never a send permit.
use super::{
    authority, funding, ledger,
    model::*,
    policy::{self, Policy, PolicyInput},
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectRequest {
    pub schema: String,
    pub policy_digest: String,
    pub allocation_hash: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingRequest {
    pub schema: String,
    pub policy_digest: String,
    pub after_allocation_hash: Option<String>,
    pub limit: u8,
}
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    Reserved,
    UserInactive,
    ProfitDeficit,
    FundingAttested,
}
#[derive(Serialize)]
pub struct Source {
    pub budget: BudgetRecord,
    pub original_settlement: Signed<Settlement>,
    pub latest_settlement: Signed<Settlement>,
    pub latest_report_digest: String,
    pub cumulative_net_profit_units: String,
    pub reserved_profit_units: String,
    pub source_status: SourceStatus,
}
#[derive(Serialize)]
pub struct SourceResponse {
    pub schema: &'static str,
    pub policy_digest: String,
    pub policy: PolicyInput,
    pub observed_at_ms: String,
    pub source: Source,
    pub offchain_payment_authorized: bool,
}
#[derive(Serialize)]
pub struct PendingResponse {
    pub schema: &'static str,
    pub policy_digest: String,
    pub policy: PolicyInput,
    pub observed_at_ms: String,
    pub sources: Vec<Source>,
    pub next_after_allocation_hash: Option<String>,
    pub offchain_payment_authorized: bool,
}
fn expected(policy: &Policy, digest: &str) -> Result<()> {
    if !policy::hex_value(digest, 32) {
        return Err(Error::Invalid.into());
    }
    if digest != policy.digest {
        return Err(Error::Conflict.into());
    }
    Ok(())
}
fn settlement(
    conn: &Connection,
    policy: &Policy,
    digest: &str,
    user: &str,
    at: i64,
) -> Result<Signed<Settlement>> {
    let json: String = conn
        .query_row(
            "SELECT proof_json FROM game_reward_reports WHERE digest=?1 AND user_id=?2",
            params![digest, user],
            |r| r.get(0),
        )
        .optional()?
        .ok_or(Error::Unavailable)?;
    let proof: Signed<Settlement> = serde_json::from_str(&json)?;
    if policy::verify(&policy.reconciler, "settlement", &proof)? != digest
        || proof.payload.schema != "esk.game.rewards.settlement.v1"
        || proof.payload.policy_digest != policy.digest
        || proof.payload.user_id != user
        || policy::natural(&proof.payload.period_end_ms)? > at
    {
        return Err(Error::Unavailable.into());
    }
    Ok(proof)
}
fn source(conn: &Connection, policy: &Policy, allocation: &str, at: i64) -> Result<Source> {
    let budget = funding::record(conn, policy, allocation)?.ok_or(Error::NotFound)?;
    let user = &budget.intent.user_id;
    if !policy::id(user) || user == "local-owner" {
        return Err(Error::Unavailable.into());
    }
    let original = settlement(conn, policy, &budget.intent.report_digest, user, at)?;
    let (latest_digest, _) = ledger::latest(conn, policy, user)?.ok_or(Error::Unavailable)?;
    let latest = settlement(conn, policy, &latest_digest, user, at)?;
    let reserved = ledger::reserved(conn, user)?;
    let net = policy::integer(&latest.payload.cumulative_net_profit_units)?;
    let amount = policy::natural(&budget.amount_units)?;
    // Verify the exact source relationship, not just independent valid signatures.
    let indexed: (String, String) = conn.query_row(
        "SELECT user_id,report_digest FROM game_reward_intents WHERE allocation_hash=?1",
        [allocation],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if indexed != (user.clone(), budget.intent.report_digest.clone())
        || original.payload.beneficiary != budget.intent.beneficiary
        || latest.payload.beneficiary != original.payload.beneficiary
        || latest.payload.wallet_binding_digest != original.payload.wallet_binding_digest
        || policy::natural(&latest.payload.sequence)? < policy::natural(&original.payload.sequence)?
        || policy::integer(&original.payload.cumulative_net_profit_units)? < amount
        || reserved < amount
    {
        return Err(Error::Unavailable.into());
    }
    let active: Option<String> = conn
        .query_row("SELECT status FROM users WHERE id=?1", [user], |r| r.get(0))
        .optional()?;
    let status = active.ok_or(Error::Unavailable)?;
    let source_status = if budget.funding.is_some() {
        SourceStatus::FundingAttested
    } else if status != "active" {
        SourceStatus::UserInactive
    } else if net < reserved {
        SourceStatus::ProfitDeficit
    } else {
        SourceStatus::Reserved
    };
    Ok(Source {
        budget,
        original_settlement: original,
        latest_settlement: latest,
        latest_report_digest: latest_digest,
        cumulative_net_profit_units: net.to_string(),
        reserved_profit_units: reserved.to_string(),
        source_status,
    })
}
fn finish(
    conn: &Connection,
    policy: &Policy,
    actor: &str,
    token: &str,
    started: i64,
    now: ledger::Clock<'_>,
) -> Result<i64> {
    let at = now()?;
    if at < started || at - started > 5000 {
        return Err(Error::Unavailable.into());
    }
    authority::session(conn, actor, token, true, at)?;
    authority::pin(conn, policy, false)?;
    Ok(at)
}
pub fn inspect(
    conn: &mut Connection,
    policy: &Policy,
    actor: &str,
    token: &str,
    request: &InspectRequest,
    now: ledger::Clock<'_>,
) -> Result<SourceResponse> {
    let at = now()?;
    // Serialize authorization against session revocation while taking a bounded snapshot.
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    authority::session(&tx, actor, token, true, at)?;
    expected(policy, &request.policy_digest)?;
    authority::pin(&tx, policy, false)?;
    if request.schema != "esk.game.rewards.funding-source.request.v1"
        || !policy::hex_value(&request.allocation_hash, 32)
    {
        return Err(Error::Invalid.into());
    }
    let source = source(&tx, policy, &request.allocation_hash, at)?;
    finish(&tx, policy, actor, token, at, now)?;
    tx.commit()?;
    Ok(SourceResponse {
        schema: "esk.game.rewards.funding-source.v1",
        policy_digest: policy.digest.clone(),
        policy: policy.input.clone(),
        observed_at_ms: at.to_string(),
        source,
        offchain_payment_authorized: false,
    })
}
pub fn pending(
    conn: &mut Connection,
    policy: &Policy,
    actor: &str,
    token: &str,
    request: &PendingRequest,
    now: ledger::Clock<'_>,
) -> Result<PendingResponse> {
    let at = now()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    authority::session(&tx, actor, token, true, at)?;
    expected(policy, &request.policy_digest)?;
    authority::pin(&tx, policy, false)?;
    if request.schema != "esk.game.rewards.pending-funding.request.v1"
        || !(1..=20).contains(&request.limit)
        || request
            .after_allocation_hash
            .as_ref()
            .is_some_and(|v| !policy::hex_value(v, 32))
    {
        return Err(Error::Invalid.into());
    }
    let ids = {
        let mut stmt = tx.prepare("SELECT i.allocation_hash FROM game_reward_intents i WHERE i.allocation_hash>?1 AND NOT EXISTS (SELECT 1 FROM game_reward_funding f WHERE f.allocation_hash=i.allocation_hash) ORDER BY i.allocation_hash LIMIT ?2")?;
        let rows = stmt.query_map(
            params![
                request.after_allocation_hash.as_deref().unwrap_or(""),
                i64::from(request.limit) + 1
            ],
            |r| r.get::<_, String>(0),
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let next_after_allocation_hash = if ids.len() > usize::from(request.limit) {
        Some(ids[usize::from(request.limit) - 1].clone())
    } else {
        None
    };
    let sources = ids
        .iter()
        .take(usize::from(request.limit))
        .map(|id| source(&tx, policy, id, at))
        .collect::<Result<Vec<_>>>()?;
    finish(&tx, policy, actor, token, at, now)?;
    tx.commit()?;
    Ok(PendingResponse {
        schema: "esk.game.rewards.pending-funding.v1",
        policy_digest: policy.digest.clone(),
        policy: policy.input.clone(),
        observed_at_ms: at.to_string(),
        sources,
        next_after_allocation_hash,
        offchain_payment_authorized: false,
    })
}
