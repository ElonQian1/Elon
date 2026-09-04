//! Pure preparation and integrity checks. No payment lookup or ledger writes.

use anyhow::Result;
use serde_json::json;

use super::model::{
    PlatformAllocationInput, PlatformError, PlatformPolicy, PolicyBody, PrepareBody, SaleTerms,
    PREPARE_SCHEMA,
};
use super::payment_identity::{
    bounded_ascii, fingerprint, normalized_source, payment_key, source_fingerprint,
};

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn base_units(value: &str, maximum: u128) -> Result<u128> {
    if value.is_empty()
        || value.len() > 39
        || value.starts_with('0')
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(PlatformError::InvalidInput.into());
    }
    value
        .parse::<u128>()
        .ok()
        .filter(|number| *number > 0 && *number <= maximum)
        .ok_or_else(|| PlatformError::InvalidInput.into())
}

/// Match the JS preview's decimal grammar, but never use floating point.
fn decimal_parts(value: &str, decimals: u32) -> Result<(&str, &str)> {
    if value.is_empty() || value.len() > 59 || decimals > 18 {
        return Err(PlatformError::InvalidInput.into());
    }
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fractional = parts.next();
    if parts.next().is_some()
        || whole.is_empty()
        || (whole.len() > 1 && whole.starts_with('0'))
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || fractional.is_some_and(str::is_empty)
    {
        return Err(PlatformError::InvalidInput.into());
    }
    let fraction = fractional.unwrap_or_default();
    if fraction.len() > decimals as usize || !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(PlatformError::InvalidInput.into());
    }
    Ok((whole, fraction))
}

fn payment_amount(value: &str, decimals: u32) -> Result<u128> {
    let (whole, fraction) = decimal_parts(value, decimals)?;
    let whole = whole
        .parse::<u128>()
        .map_err(|_| PlatformError::InvalidInput)?;
    let fraction_value = if fraction.is_empty() {
        0
    } else {
        fraction
            .parse::<u128>()
            .map_err(|_| PlatformError::InvalidInput)?
    };
    whole
        .checked_mul(10_u128.pow(decimals))
        .and_then(|whole| {
            fraction_value
                .checked_mul(10_u128.pow(decimals - fraction.len() as u32))
                .and_then(|fraction| whole.checked_add(fraction))
        })
        .filter(|number| *number > 0)
        .ok_or_else(|| PlatformError::InvalidInput.into())
}

pub(crate) fn validate_policy(body: PolicyBody) -> Result<PlatformPolicy> {
    let source = normalized_source(&body.source).map_err(|_| PlatformError::InvalidPolicy)?;
    let limit = base_units(&body.issuance_limit_base_units, i64::MAX as u128)
        .map_err(|_| PlatformError::InvalidPolicy)? as i64;
    let source_fingerprint =
        source_fingerprint(&source).map_err(|_| PlatformError::InvalidPolicy)?;
    let policy_digest = fingerprint(&json!({
        "schema": "yilong.esk.platform_policy.v1",
        "source": source,
        "issuance_limit_base_units": limit.to_string(),
    }))
    .map_err(|_| PlatformError::InvalidPolicy)?;
    Ok(PlatformPolicy {
        source,
        source_fingerprint,
        policy_digest,
        issuance_limit_base_units: limit,
    })
}

pub(crate) fn policy_from_values(
    mode: Option<&str>,
    policy_json: Option<&str>,
) -> Result<PlatformPolicy> {
    match mode {
        None | Some("disabled") => return Err(PlatformError::Disabled.into()),
        Some("platform_recorded") => (),
        _ => return Err(PlatformError::InvalidPolicy.into()),
    }
    let raw = policy_json
        .filter(|value| !value.is_empty() && value.len() <= 4096)
        .ok_or(PlatformError::InvalidPolicy)?;
    // Derive rejects duplicate known fields, unknown fields, wrong types and trailing JSON.
    let body = serde_json::from_str::<PolicyBody>(raw).map_err(|_| PlatformError::InvalidPolicy)?;
    validate_policy(body)
}

pub(crate) fn load_policy() -> Result<PlatformPolicy> {
    fn variable(name: &str) -> Result<Option<String>> {
        match std::env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(std::env::VarError::NotUnicode(_)) => Err(PlatformError::InvalidPolicy.into()),
        }
    }
    let mode = variable("ESK_PLATFORM_ASSET_MODE")?;
    let policy = variable("ESK_PLATFORM_ASSET_POLICY")?;
    policy_from_values(mode.as_deref(), policy.as_deref())
}

fn validate_policy_integrity(policy: &PlatformPolicy) -> Result<()> {
    let actual = validate_policy(PolicyBody {
        source: policy.source.clone(),
        issuance_limit_base_units: policy.issuance_limit_base_units.to_string(),
    })?;
    if actual.source != policy.source
        || actual.source_fingerprint != policy.source_fingerprint
        || actual.policy_digest != policy.policy_digest
    {
        return Err(PlatformError::InvalidPolicy.into());
    }
    Ok(())
}

fn sale_terms(sale: &SaleTerms, payment: u128, amount: i64) -> Result<String> {
    if !bounded_ascii(&sale.sale_batch_id, 80, false)
        || !bounded_ascii(&sale.disclosure_revision, 80, false)
        || !digest(&sale.terms_digest)
    {
        return Err(PlatformError::InvalidInput.into());
    }
    let denominator = base_units(&sale.payment_base_units_per_lot, u128::MAX)?;
    let numerator = base_units(&sale.esk_base_units_per_lot, i64::MAX as u128)?;
    // Reduce first: a legitimate u128 payment can otherwise overflow an intermediate product.
    let (mut left, mut right) = (payment, denominator);
    while right != 0 {
        (left, right) = (right, left % right);
    }
    let reduced_denominator = denominator / left;
    if numerator % reduced_denominator != 0
        || (payment / left).checked_mul(numerator / reduced_denominator) != Some(amount as u128)
    {
        return Err(PlatformError::InvalidInput.into());
    }
    fingerprint(&json!({
        "schema": "yilong.esk.platform_sale_terms.v1",
        "sale_batch_id": sale.sale_batch_id,
        "payment_base_units_per_lot": denominator.to_string(),
        "esk_base_units_per_lot": numerator.to_string(),
        "disclosure_revision": sale.disclosure_revision,
        "terms_digest": sale.terms_digest,
    }))
}

fn request_digest(input: &PlatformAllocationInput) -> Result<String> {
    let mut value = serde_json::to_value(input).map_err(|_| PlatformError::InvalidInput)?;
    let fields = value.as_object_mut().ok_or(PlatformError::InvalidInput)?;
    fields.remove("request_digest");
    fields.insert(
        "schema".into(),
        json!("yilong.esk.platform_allocation_request.v1"),
    );
    fields.insert("commercial_purpose".into(), json!("esk_purchase"));
    fields.insert("history_complete".into(), json!(true));
    fingerprint(&value)
}

pub(crate) fn prepare_input(
    policy: &PlatformPolicy,
    body: PrepareBody,
) -> Result<PlatformAllocationInput> {
    validate_policy_integrity(policy)?;
    if body.schema != PREPARE_SCHEMA
        || body.commercial_purpose != "esk_purchase"
        || !body.history_complete
        || !bounded_ascii(&body.user_id, 80, false)
        || !bounded_ascii(&body.review_reference, 80, false)
        || !digest(&body.payment_evidence_digest)
        || !digest(&body.consent_digest)
        || !digest(&body.history_evidence_digest)
    {
        return Err(PlatformError::InvalidInput.into());
    }
    decimal_parts(&body.amount, 6)?;
    let amount = crate::esk_asset::parse_esk_amount(&body.amount)
        .map_err(|_| PlatformError::InvalidInput)?;
    let payment = payment_amount(&body.payment_amount, policy.source.decimals)?;
    let payment_key = payment_key(
        &policy.source,
        &body.external_payment_reference,
        body.transfer_index,
    )?;
    let sale_terms_digest = sale_terms(&body.sale, payment, amount)?;
    let review_reference_digest = fingerprint(&json!({
        "schema": "yilong.esk.platform_review_reference.v1",
        "review_reference": body.review_reference,
    }))?;
    let mut input = PlatformAllocationInput {
        user_id: body.user_id,
        source_fingerprint: policy.source_fingerprint.clone(),
        policy_digest: policy.policy_digest.clone(),
        payment_key,
        payment_base_units: payment.to_string(),
        amount_base_units: amount,
        sale_terms_digest,
        payment_evidence_digest: body.payment_evidence_digest,
        consent_digest: body.consent_digest,
        history_evidence_digest: body.history_evidence_digest,
        review_reference_digest,
        request_digest: String::new(),
    };
    input.request_digest = request_digest(&input)?;
    validate_prepared_input(policy, &input)?;
    Ok(input)
}

/// Recheck stored integrity, not external truth or the original unhashed sale materials.
pub(crate) fn validate_prepared_input(
    policy: &PlatformPolicy,
    input: &PlatformAllocationInput,
) -> Result<()> {
    validate_policy_integrity(policy)?;
    if !bounded_ascii(&input.user_id, 80, false)
        || input.source_fingerprint != policy.source_fingerprint
        || input.policy_digest != policy.policy_digest
        || input.amount_base_units <= 0
        || ![
            &input.payment_key,
            &input.sale_terms_digest,
            &input.payment_evidence_digest,
            &input.consent_digest,
            &input.history_evidence_digest,
            &input.review_reference_digest,
            &input.request_digest,
        ]
        .iter()
        .all(|value| digest(value))
    {
        return Err(PlatformError::InvalidInput.into());
    }
    base_units(&input.payment_base_units, u128::MAX)?;
    if input.amount_base_units > policy.issuance_limit_base_units {
        return Err(PlatformError::LimitExceeded.into());
    }
    if input.request_digest != request_digest(input)? {
        return Err(PlatformError::InvalidInput.into());
    }
    Ok(())
}
