use crate::{model::*, settlement::Settlement, verify::*};
use anyhow::{ensure, Result};

fn calculation(c: &Config, source_digest: String, s: &Statement) -> Result<Calculation> {
    // Wider intermediates permit offsetting amounts while the wire/SQLite result stays i64.
    let net = i128::from(integer(&s.cumulative_realized_trading_pnl_units)?)
        + i128::from(integer(&s.cumulative_net_funding_units)?)
        - i128::from(natural(&s.cumulative_trading_fees_units)?)
        - i128::from(natural(&s.cumulative_other_costs_units)?)
        - i128::from(natural(&s.held_reserve_units)?);
    let net = i64::try_from(net)?;
    Ok(Calculation {
        schema: "esk.game.reconciliation.calculation.v1",
        source_public_key_hex: c.source_public_key_hex.clone(),
        source_digest,
        cumulative_realized_trading_pnl_units: s.cumulative_realized_trading_pnl_units.clone(),
        cumulative_trading_fees_units: s.cumulative_trading_fees_units.clone(),
        cumulative_net_funding_units: s.cumulative_net_funding_units.clone(),
        cumulative_other_costs_units: s.cumulative_other_costs_units.clone(),
        held_reserve_units: s.held_reserve_units.clone(),
        cumulative_net_profit_units: net.to_string(),
        excluded_principal_liability_units: s.principal_liability_units.clone(),
        excluded_unrealized_pnl_units: s.unrealized_pnl_units.clone(),
    })
}
fn settlement(
    s: &Statement,
    calculation: &Calculation,
    previous_report_digest: String,
) -> Result<Settlement> {
    Ok(Settlement {
        schema: "esk.game.rewards.settlement.v1".into(),
        policy_digest: s.policy_digest.clone(),
        user_id: s.user_id.clone(),
        beneficiary: s.beneficiary.clone(),
        wallet_binding_digest: s.wallet_binding_digest.clone(),
        sequence: s.sequence.clone(),
        previous_report_digest,
        period_end_ms: s.period_end_ms.clone(),
        cumulative_net_profit_units: calculation.cumulative_net_profit_units.clone(),
        reconciliation_digest: hash(canonical(CALCULATION_DOMAIN, calculation)?),
    })
}

pub fn prepare(c: &Config, request: &Request, now_ms: i64) -> Result<Candidate> {
    config(c)?;
    ensure!(
        request.schema == "esk.game.reconciliation.request.v1",
        "unsupported request"
    );
    let digest = statement(c, &request.statement, now_ms)?;
    let current = &request.statement.payload;
    let previous_report_digest = match &request.previous {
        None => {
            ensure!(
                natural(&current.sequence)? == 1,
                "previous signed settlement required"
            );
            "0".repeat(64)
        }
        Some(previous) => previous_digest(c, current, previous, now_ms)?,
    };
    let calculation = calculation(c, digest, current)?;
    let settlement = settlement(current, &calculation, previous_report_digest)?;
    let settlement_signing_bytes_hex = hex::encode(canonical(SETTLEMENT_DOMAIN, &settlement)?);
    Ok(Candidate {
        schema: "esk.game.reconciliation.candidate.v1",
        statement: request.statement.clone(),
        calculation,
        settlement,
        settlement_signing_bytes_hex,
        settlement_signature_present: false,
        external_facts_independently_verified: false,
        offchain_payment_authorized: false,
        funds_moved: false,
    })
}

fn previous_digest(
    c: &Config,
    current: &Statement,
    previous: &Previous,
    now_ms: i64,
) -> Result<String> {
    let source_digest = statement(c, &previous.statement, now_ms)?;
    let prior = &previous.statement.payload;
    ensure!(
        natural(&prior.sequence)?.checked_add(1) == Some(natural(&current.sequence)?)
            && current.previous_source_digest == source_digest
            && current.period_start_ms == prior.period_end_ms,
        "accounting chain discontinuity"
    );
    ensure!(
        natural(&current.cumulative_trading_fees_units)?
            >= natural(&prior.cumulative_trading_fees_units)?
            && natural(&current.cumulative_other_costs_units)?
                >= natural(&prior.cumulative_other_costs_units)?,
        "cumulative expense reversed"
    );
    let proof = &previous.settlement;
    let report_digest = signed(&c.reconciler_public_key_hex, SETTLEMENT_DOMAIN, proof)?;
    let parent = &proof.payload.previous_report_digest;
    ensure!(fixed_hex(parent, 32), "invalid previous report parent");
    ensure!(
        (prior.sequence == "1") == (parent == &"0".repeat(64)),
        "invalid previous report ancestry"
    );
    let calculation = calculation(c, source_digest, prior)?;
    ensure!(
        proof.payload == settlement(prior, &calculation, parent.clone())?,
        "previous settlement differs from source accounting"
    );
    Ok(report_digest)
}
