use anyhow::{bail, Result};
use rusqlite::{params, Transaction};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::task_settlement::{
    ledger::{compute_mirror_postings, ensure_balanced, LedgerPosting},
    model::SettlementCorrection,
};

use super::{new_id, task_settlement_correction_rows::digest_text};

pub(super) const REVERSAL_POLICY: &str = "task-shadow-correction-reversal.v1";
pub(super) const REPLACEMENT_POLICY: &str = "task-shadow-correction-replacement.v1";

pub(super) struct CorrectionLeg<'a> {
    pub(super) key: &'a str,
    pub(super) policy: &'a str,
    pub(super) receipt_kind: &'a str,
    pub(super) compute_amount_micros: i64,
    pub(super) provider_amount_micros: i64,
    pub(super) platform_amount_micros: i64,
    pub(super) reverse_postings: bool,
}

pub(super) fn insert_correction_leg(
    tx: &Transaction<'_>,
    correction: &SettlementCorrection,
    currency: &str,
    payer_user_id: &str,
    payee_user_id: Option<&str>,
    leg: &CorrectionLeg<'_>,
    timestamp: &str,
) -> Result<String> {
    let usage_id = new_id("usage");
    let intent_id = new_id("intent");
    let receipt_id = new_id("settlement");
    let source_id = format!("{}:{}", correction.id, leg.key);
    let source_digest = digest_json(&json!({
        "schema": "task_usage.settlement_correction.v1",
        "correction_id": correction.id,
        "dispute_id": correction.dispute_id,
        "original_receipt_id": correction.original_settlement_receipt_id,
        "leg": leg.key,
        "compute_amount_micros": leg.compute_amount_micros,
        "provider_amount_micros": leg.provider_amount_micros,
        "currency": currency,
    }))?;
    tx.execute(
        "INSERT INTO task_usage_receipts (
           id, project_id, subject_type, subject_id, source_type, source_id,
           source_digest, consumer_user_id, provider_user_id, units,
           amount_micros, provider_amount_micros, currency, billing_source,
           source_status, occurred_at, created_at
         ) VALUES (?1, ?2, 'settlement_correction', ?3, 'settlement_correction', ?4,
           ?5, ?6, ?7, 0, ?8, ?9, ?10, 'accepted_dispute_correction',
           'reconciled', ?11, ?11)",
        params![
            usage_id,
            correction.project_id,
            correction.id,
            source_id,
            source_digest,
            payer_user_id,
            payee_user_id,
            leg.compute_amount_micros,
            leg.provider_amount_micros,
            currency,
            timestamp,
        ],
    )?;
    let idempotency_key = format!("task-shadow-correction:v1:{}:{}", correction.id, leg.key);
    tx.execute(
        "INSERT INTO task_settlement_intents (
           id, project_id, matter_id, assignment_id, payer_user_id, payee_user_id,
           idempotency_key, policy_version, policy_digest, status, shadow_only,
           created_at, updated_at
         ) VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8, 'posted', 1, ?9, ?9)",
        params![
            intent_id,
            correction.project_id,
            correction.correction_matter_id,
            payer_user_id,
            payee_user_id,
            idempotency_key,
            leg.policy,
            digest_text(leg.policy),
            timestamp,
        ],
    )?;
    tx.execute(
        "INSERT INTO task_settlement_intent_sources (intent_id, usage_receipt_id, created_at)
         VALUES (?1, ?2, ?3)",
        params![intent_id, usage_id, timestamp],
    )?;
    let posting_key = format!(
        "task-shadow-correction-post:v1:{}:{}",
        correction.id, leg.key
    );
    tx.execute(
        "INSERT INTO task_settlement_receipts (
           id, project_id, intent_id, posting_key, status,
           compute_amount_micros, provider_amount_micros, platform_amount_micros,
           outcome_reward_micros, review_reward_micros, currency, shadow_only,
           accepted_matter_id, reason, receipt_kind, correction_id, created_at
         ) VALUES (?1, ?2, ?3, ?4, 'reconciled', ?5, ?6, ?7, 0, 0, ?8, 1,
           ?9, ?10, ?11, ?12, ?13)",
        params![
            receipt_id,
            correction.project_id,
            intent_id,
            posting_key,
            leg.compute_amount_micros,
            leg.provider_amount_micros,
            leg.platform_amount_micros,
            currency,
            correction.correction_matter_id,
            if leg.reverse_postings {
                "accepted dispute correction: reverse original shadow receipt"
            } else {
                "accepted dispute correction: append replacement shadow receipt"
            },
            leg.receipt_kind,
            correction.id,
            timestamp,
        ],
    )?;
    let mut postings = compute_mirror_postings(
        payer_user_id,
        payee_user_id,
        leg.compute_amount_micros,
        leg.provider_amount_micros,
    )?;
    if leg.reverse_postings {
        for posting in &mut postings {
            posting.side = match posting.side {
                "debit" => "credit",
                "credit" => "debit",
                _ => bail!("未知影子账本方向"),
            };
        }
    }
    insert_ledger_transaction(
        tx,
        &correction.project_id,
        &receipt_id,
        &posting_key,
        leg,
        &postings,
        currency,
        timestamp,
    )?;
    Ok(receipt_id)
}

fn insert_ledger_transaction(
    tx: &Transaction<'_>,
    project_id: &str,
    receipt_id: &str,
    posting_key: &str,
    leg: &CorrectionLeg<'_>,
    postings: &[LedgerPosting],
    currency: &str,
    timestamp: &str,
) -> Result<()> {
    ensure_balanced(postings)?;
    if postings.is_empty() {
        return Ok(());
    }
    let transaction_id = new_id("ledger");
    tx.execute(
        "INSERT INTO task_ledger_transactions (
           id, project_id, settlement_receipt_id, posting_key, description, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            transaction_id,
            project_id,
            receipt_id,
            posting_key,
            if leg.reverse_postings {
                "accepted dispute correction reversal"
            } else {
                "accepted dispute correction replacement"
            },
            timestamp,
        ],
    )?;
    for posting in postings {
        tx.execute(
            "INSERT INTO task_ledger_entries (
               id, transaction_id, account_key, user_id, side,
               amount_micros, currency, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                new_id("entry"),
                transaction_id,
                posting.account_key,
                posting.user_id,
                posting.side,
                posting.amount_micros,
                currency,
                timestamp,
            ],
        )?;
    }
    Ok(())
}

fn digest_json(value: &serde_json::Value) -> Result<String> {
    Ok(hex::encode(Sha256::digest(
        serde_json::to_string(value)?.as_bytes(),
    )))
}
