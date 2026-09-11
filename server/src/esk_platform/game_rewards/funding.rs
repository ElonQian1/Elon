use super::{
    authority, ledger,
    model::*,
    policy::{self, Policy},
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

pub fn record(
    conn: &Connection,
    policy: &Policy,
    allocation: &str,
) -> Result<Option<BudgetRecord>> {
    let row: Option<(String,i64,String)> = conn.query_row("SELECT digest,amount_units,intent_json FROM game_reward_intents WHERE allocation_hash=?1", [allocation], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
    row.map(|(digest,amount,json)| {
        let intent: BudgetIntent = serde_json::from_str(&json)?;
        if intent.allocation_hash != allocation || ledger::amount(&intent,policy)? != amount
            || policy::hash(policy::canonical("intent",&intent)?) != digest { return Err(Error::Unavailable.into()); }
        let proof: Option<(String,String)> = conn.query_row("SELECT evidence_digest,proof_json FROM game_reward_funding WHERE allocation_hash=?1", [allocation], |r| Ok((r.get(0)?,r.get(1)?))).optional()?;
        let funding = proof.map(|(hash,json)| -> Result<FundingEvidence> {
            let proof: Signed<FundingEvidence> = serde_json::from_str(&json)?;
            if policy::verify(&policy.observer,"funding",&proof)? != hash { return Err(Error::Unavailable.into()); }
            matches(&proof.payload,policy,&intent,&digest,amount)?;
            Ok(proof.payload)
        }).transpose()?;
        Ok(BudgetRecord { intent, intent_digest:digest, amount_units:amount.to_string(), state:if funding.is_some() { "funding_attested" } else { "reserved_awaiting_funding" }, funding, replayed:false, offchain_payment_authorized:false })
    }).transpose()
}
fn matches(
    proof: &FundingEvidence,
    policy: &Policy,
    intent: &BudgetIntent,
    digest: &str,
    amount: i64,
) -> Result<()> {
    if proof.schema != "esk.game.rewards.funding.v1"
        || proof.policy_digest != policy.digest
        || proof.intent_digest != digest
        || proof.allocation_hash != intent.allocation_hash
        || proof.beneficiary != intent.beneficiary
        || policy::natural(&proof.amount_units)? != amount
        || !policy::address(&proof.budget_id)
        || !policy::chain_digest(&proof.transaction_digest)
        || !policy::chain_digest(&proof.checkpoint_digest)
        || policy::natural(&proof.checkpoint)? == 0
        || policy::natural(&proof.checkpoint_time_ms)? == 0
        || policy::natural(&proof.observed_at_ms)? < policy::natural(&proof.checkpoint_time_ms)?
        || !proof.initial_budget_verified
    {
        return Err(Error::Invalid.into());
    }
    Ok(())
}
pub fn confirm(
    conn: &mut Connection,
    policy: &Policy,
    actor: &str,
    token: &str,
    proof: &Signed<FundingEvidence>,
    now: ledger::Clock<'_>,
) -> Result<BudgetRecord> {
    let at = now()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    authority::session(&tx, actor, token, true, at)?;
    authority::pin(&tx, policy, false)?;
    let digest = policy::verify(&policy.observer, "funding", proof)?;
    let mut current =
        record(&tx, policy, &proof.payload.allocation_hash)?.ok_or(Error::NotFound)?;
    authority::user(&tx, &current.intent.user_id)?;
    matches(
        &proof.payload,
        policy,
        &current.intent,
        &current.intent_digest,
        policy::natural(&current.amount_units)?,
    )?;
    if policy::natural(&proof.payload.observed_at_ms)? > at {
        return Err(Error::Invalid.into());
    }
    if let Some(existing) = &current.funding {
        if existing != &proof.payload {
            return Err(Error::Conflict.into());
        }
        current.replayed = true;
    } else {
        let used: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM game_reward_funding WHERE budget_id=?1)",
            [&proof.payload.budget_id],
            |r| r.get(0),
        )?;
        if used {
            return Err(Error::Conflict.into());
        }
        tx.execute(
            "INSERT INTO game_reward_funding VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                proof.payload.allocation_hash,
                proof.payload.budget_id,
                digest,
                serde_json::to_string(proof)?,
                actor,
                at
            ],
        )?;
        current = record(&tx, policy, &proof.payload.allocation_hash)?.ok_or(Error::Unavailable)?;
    }
    authority::session(&tx, actor, token, true, now()?)?;
    tx.commit()?;
    Ok(current)
}
pub fn account(
    conn: &mut Connection,
    policy: &Policy,
    user: &str,
    token: &str,
    now: ledger::Clock<'_>,
) -> Result<AccountView> {
    let tx = conn.transaction()?;
    authority::session(&tx, user, token, false, now()?)?;
    authority::pin(&tx, policy, false)?;
    let latest = ledger::latest(&tx, policy, user)?;
    let net = latest
        .as_ref()
        .map(|(_, s)| policy::integer(&s.cumulative_net_profit_units))
        .transpose()?
        .unwrap_or(0);
    let used = ledger::reserved(&tx, user)?;
    let available = if net > used { net - used } else { 0 };
    let ids: Vec<String> = {
        let mut query = tx.prepare("SELECT allocation_hash FROM game_reward_intents WHERE user_id=?1 ORDER BY recorded_ms DESC, allocation_hash LIMIT 100")?;
        let rows = query.query_map([user], |r| r.get(0))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    let budgets = ids
        .iter()
        .map(|id| record(&tx, policy, id)?.ok_or_else(|| Error::Unavailable.into()))
        .collect::<Result<Vec<_>>>()?;
    authority::session(&tx, user, token, false, now()?)?;
    tx.commit()?;
    Ok(AccountView {
        schema: "esk.game.rewards.account.v1",
        policy_digest: policy.digest.clone(),
        asset_type: policy.input.asset_type.clone(),
        latest_report_digest: latest.map(|(d, _)| d),
        cumulative_net_profit_units: net.to_string(),
        reserved_profit_units: used.to_string(),
        available_profit_units: available.to_string(),
        budgets,
        offchain_payment_authorized: false,
    })
}
