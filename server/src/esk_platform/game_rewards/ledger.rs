use super::{
    authority,
    model::*,
    policy::{self, Policy},
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

pub type Clock<'a> = &'a dyn Fn() -> Result<i64>;
pub fn latest(
    conn: &Connection,
    policy: &Policy,
    user: &str,
) -> Result<Option<(String, Settlement)>> {
    let row: Option<(String,String)> = conn.query_row("SELECT digest,proof_json FROM game_reward_reports WHERE user_id=?1 ORDER BY sequence DESC LIMIT 1", [user], |r| Ok((r.get(0)?,r.get(1)?))).optional()?;
    row.map(|(digest, json)| {
        let proof: Signed<Settlement> = serde_json::from_str(&json)?;
        if proof.payload.user_id != user
            || proof.payload.policy_digest != policy.digest
            || policy::verify(&policy.reconciler, "settlement", &proof)? != digest
        {
            return Err(Error::Unavailable.into());
        }
        Ok((digest, proof.payload))
    })
    .transpose()
}
pub fn reserved(conn: &Connection, user: &str) -> Result<i64> {
    let value: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_units),0) FROM game_reward_intents WHERE user_id=?1",
        [user],
        |r| r.get(0),
    )?;
    if value < 0 {
        return Err(Error::Unavailable.into());
    }
    Ok(value)
}
pub fn record_settlement(
    conn: &mut Connection,
    policy: &Policy,
    actor: &str,
    token: &str,
    proof: &Signed<Settlement>,
    now: Clock<'_>,
) -> Result<String> {
    let at = now()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    authority::session(&tx, actor, token, true, at)?;
    let digest = policy::verify(&policy.reconciler, "settlement", proof)?;
    let s = &proof.payload;
    let sequence = policy::natural(&s.sequence)?;
    let end = policy::natural(&s.period_end_ms)?;
    let net = policy::integer(&s.cumulative_net_profit_units)?;
    if s.schema != "esk.game.rewards.settlement.v1"
        || s.policy_digest != policy.digest
        || !policy::address(&s.beneficiary)
        || !policy::hex_value(&s.wallet_binding_digest, 32)
        || !policy::hex_value(&s.reconciliation_digest, 32)
        || !policy::hex_value(&s.previous_report_digest, 32)
        || sequence == 0
        || end == 0
        || end > at
    {
        return Err(Error::Invalid.into());
    }
    authority::user(&tx, &s.user_id)?;
    authority::pin(&tx, policy, true)?;
    let existing: Option<String> = tx
        .query_row(
            "SELECT proof_json FROM game_reward_reports WHERE digest=?1",
            [&digest],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(existing) = existing {
        if serde_json::from_str::<Signed<Settlement>>(&existing)? != *proof {
            return Err(Error::Conflict.into());
        }
    } else {
        match latest(&tx, policy, &s.user_id)? {
            Some((previous, last))
                if s.previous_report_digest == previous
                    && policy::natural(&last.sequence)?.checked_add(1) == Some(sequence)
                    && policy::natural(&last.period_end_ms)? < end
                    && last.beneficiary == s.beneficiary
                    && last.wallet_binding_digest == s.wallet_binding_digest =>
            {
                ()
            }
            None if sequence == 1 && s.previous_report_digest == "0".repeat(64) => (),
            _ => return Err(Error::Conflict.into()),
        }
        tx.execute(
            "INSERT INTO game_reward_reports VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                digest,
                s.user_id,
                policy.digest,
                sequence,
                s.previous_report_digest,
                s.beneficiary,
                end,
                net,
                serde_json::to_string(proof)?,
                actor,
                at
            ],
        )?;
    }
    authority::session(&tx, actor, token, true, now()?)?;
    tx.commit()?;
    Ok(digest)
}
pub fn amount(intent: &BudgetIntent, policy: &Policy) -> Result<i64> {
    if intent.schema != "esk.game.rewards.intent.v1"
        || intent.policy_digest != policy.digest
        || !policy::hex_value(&intent.allocation_hash, 32)
        || !policy::hex_value(&intent.report_digest, 32)
        || !policy::hex_value(&intent.content_hash, 32)
        || !policy::hex_value(&intent.license_hash, 32)
        || !policy::address(&intent.beneficiary)
        || !policy::address(&intent.mint_authority)
        || !policy::address(&intent.creator)
        || intent.denominations.len() != 6
        || intent.tickets.len() != 6
        || policy::natural(&intent.direct_claim_after_ms)? < policy::natural(&intent.opens_at_ms)?
    {
        return Err(Error::Invalid.into());
    }
    let mut total = 0i64;
    let mut count = 0i64;
    let mut previous = 0i64;
    for (denomination, tickets) in intent.denominations.iter().zip(&intent.tickets) {
        let d = policy::natural(denomination)?;
        let n = policy::natural(tickets)?;
        if d <= previous {
            return Err(Error::Invalid.into());
        }
        previous = d;
        count = count.checked_add(n).ok_or(Error::Invalid)?;
        total = total
            .checked_add(d.checked_mul(n).ok_or(Error::Invalid)?)
            .ok_or(Error::Invalid)?;
    }
    if total == 0 || count > 10000 {
        return Err(Error::Invalid.into());
    }
    Ok(total)
}
pub fn prepare(
    conn: &mut Connection,
    policy: &Policy,
    actor: &str,
    token: &str,
    intent: &BudgetIntent,
    now: Clock<'_>,
) -> Result<BudgetRecord> {
    let at = now()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    authority::session(&tx, actor, token, true, at)?;
    authority::user(&tx, &intent.user_id)?;
    authority::pin(&tx, policy, false)?;
    let total = amount(intent, policy)?;
    let digest = policy::hash(policy::canonical("intent", intent)?);
    if let Some(mut existing) = super::funding::record(&tx, policy, &intent.allocation_hash)? {
        if existing.intent_digest != digest || existing.intent != *intent {
            return Err(Error::Conflict.into());
        }
        existing.replayed = true;
        authority::session(&tx, actor, token, true, now()?)?;
        tx.commit()?;
        return Ok(existing);
    }
    let (report_digest, last) = latest(&tx, policy, &intent.user_id)?.ok_or(Error::Insufficient)?;
    if report_digest != intent.report_digest || last.beneficiary != intent.beneficiary {
        return Err(Error::Conflict.into());
    }
    let net = policy::integer(&last.cumulative_net_profit_units)?;
    let used = reserved(&tx, &intent.user_id)?;
    if net <= used || total > net - used {
        return Err(Error::Insufficient.into());
    }
    tx.execute(
        "INSERT INTO game_reward_intents VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            intent.allocation_hash,
            digest,
            intent.user_id,
            intent.report_digest,
            total,
            serde_json::to_string(intent)?,
            actor,
            at
        ],
    )?;
    let result =
        super::funding::record(&tx, policy, &intent.allocation_hash)?.ok_or(Error::Unavailable)?;
    authority::session(&tx, actor, token, true, now()?)?;
    tx.commit()?;
    Ok(result)
}
