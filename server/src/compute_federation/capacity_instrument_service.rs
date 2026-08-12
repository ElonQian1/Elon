//! Platform-administrator orchestration for standardized capacity instruments.

use anyhow::Error as AnyError;
use serde::Deserialize;
use thiserror::Error;

use crate::{
    compute_federation::{
        capacity_instrument::{
            ComputeCapacityInstrumentContractUnit,
            COMPUTE_CAPACITY_INSTRUMENT_ACTIVATION_CONFIRMATION,
            COMPUTE_CAPACITY_INSTRUMENT_OFFER_ADOPTION_CONFIRMATION,
            COMPUTE_CAPACITY_INSTRUMENT_REGISTRATION_CONFIRMATION,
            COMPUTE_CAPACITY_INSTRUMENT_RETIREMENT_CONFIRMATION,
            COMPUTE_CAPACITY_INSTRUMENT_REVISION, COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_CURRENCY,
            COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_UNIT,
        },
        market::ComputeDeliveryWindow,
    },
    store::{
        ActivateComputeCapacityInstrument, AdoptComputeCapacityInstrumentOffer,
        ComputeCapacityInstrumentActivationWriteReceipt,
        ComputeCapacityInstrumentCurrentnessReceipt,
        ComputeCapacityInstrumentOfferAdoptionWriteReceipt,
        ComputeCapacityInstrumentRegistrationWriteReceipt,
        ComputeCapacityInstrumentRetirementWriteReceipt, RegisterComputeCapacityInstrument,
        RetireComputeCapacityInstrument, Store,
    },
};

const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterComputeCapacityInstrumentBody {
    pub instrument_id: String,
    pub sku_id: String,
    pub sku_digest: String,
    pub delivery_window: ComputeDeliveryWindow,
    pub contract_units: Vec<ComputeCapacityInstrumentContractUnit>,
    pub availability_sla_tier: String,
    pub region_or_data_zone: String,
    pub verification_tier: String,
    pub settlement_currency: String,
    pub settlement_unit: String,
    pub idempotency_key: String,
    pub confirm_registration: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActivateComputeCapacityInstrumentBody {
    pub expected_instrument_revision: i64,
    pub expected_instrument_digest: String,
    pub idempotency_key: String,
    pub confirm_activation: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RetireComputeCapacityInstrumentBody {
    pub expected_instrument_revision: i64,
    pub expected_instrument_digest: String,
    pub reason: String,
    pub idempotency_key: String,
    pub confirm_retirement: bool,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AdoptComputeCapacityInstrumentOfferBody {
    pub instrument_id: String,
    pub expected_instrument_revision: i64,
    pub expected_instrument_digest: String,
    pub expected_offer_version: i64,
    pub expected_offer_digest: String,
    pub expected_publication_id: String,
    pub expected_publication_digest: String,
    pub idempotency_key: String,
    pub confirm_adoption: bool,
}

#[derive(Debug, Error)]
pub(crate) enum ComputeCapacityInstrumentServiceError {
    #[error("capacity instrument was not found")]
    NotFound,
    #[error("capacity-instrument request is invalid")]
    Invalid(#[source] AnyError),
    #[error("capacity-instrument state conflicts with immutable history")]
    Conflict(#[source] AnyError),
}

pub(crate) fn register_for_admin(
    store: &Store,
    admin_user_id: &str,
    body: RegisterComputeCapacityInstrumentBody,
) -> Result<ComputeCapacityInstrumentRegistrationWriteReceipt, ComputeCapacityInstrumentServiceError>
{
    if !body.confirm_registration
        || body.settlement_currency != COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_CURRENCY
        || body.settlement_unit != COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_UNIT
    {
        return Err(invalid("登记容量工具前必须确认并使用冻结的 CNY 结算合同"));
    }
    validate_register_body(&body)?;
    store
        .register_compute_capacity_instrument(RegisterComputeCapacityInstrument {
            instrument_id: body.instrument_id,
            sku_id: body.sku_id,
            sku_digest: body.sku_digest,
            delivery_window: body.delivery_window,
            contract_units: body.contract_units,
            availability_sla_tier: body.availability_sla_tier,
            region_or_data_zone: body.region_or_data_zone,
            verification_tier: body.verification_tier,
            settlement_currency: body.settlement_currency,
            settlement_unit: body.settlement_unit,
            registered_by_admin_user_id: admin_user_id.to_string(),
            confirmation: COMPUTE_CAPACITY_INSTRUMENT_REGISTRATION_CONFIRMATION.to_string(),
            idempotency_scope: operation_scope("register", admin_user_id),
            idempotency_key: body.idempotency_key,
        })
        .map_err(classify_store_error)
}

pub(crate) fn activate_for_admin(
    store: &Store,
    admin_user_id: &str,
    instrument_id: &str,
    body: ActivateComputeCapacityInstrumentBody,
) -> Result<ComputeCapacityInstrumentActivationWriteReceipt, ComputeCapacityInstrumentServiceError>
{
    if !body.confirm_activation
        || body.expected_instrument_revision != COMPUTE_CAPACITY_INSTRUMENT_REVISION
    {
        return Err(invalid("激活容量工具前必须确认冻结的工具版本"));
    }
    validate_expected_identity(
        instrument_id,
        body.expected_instrument_revision,
        &body.expected_instrument_digest,
    )?;
    require_instrument(store, instrument_id)?;
    store
        .activate_compute_capacity_instrument(ActivateComputeCapacityInstrument {
            instrument_id: instrument_id.to_string(),
            expected_instrument_revision: body.expected_instrument_revision,
            expected_instrument_digest: body.expected_instrument_digest,
            activated_by_admin_user_id: admin_user_id.to_string(),
            confirmation: COMPUTE_CAPACITY_INSTRUMENT_ACTIVATION_CONFIRMATION.to_string(),
            idempotency_scope: operation_scope("activate", admin_user_id),
            idempotency_key: body.idempotency_key,
        })
        .map_err(classify_store_error)
}

pub(crate) fn retire_for_admin(
    store: &Store,
    admin_user_id: &str,
    instrument_id: &str,
    body: RetireComputeCapacityInstrumentBody,
) -> Result<ComputeCapacityInstrumentRetirementWriteReceipt, ComputeCapacityInstrumentServiceError>
{
    if !body.confirm_retirement
        || body.expected_instrument_revision != COMPUTE_CAPACITY_INSTRUMENT_REVISION
    {
        return Err(invalid("退役容量工具前必须确认冻结的工具版本"));
    }
    validate_expected_identity(
        instrument_id,
        body.expected_instrument_revision,
        &body.expected_instrument_digest,
    )?;
    validate_text(&body.reason, "退役原因", 8, 2_000)?;
    require_instrument(store, instrument_id)?;
    store
        .retire_compute_capacity_instrument(RetireComputeCapacityInstrument {
            instrument_id: instrument_id.to_string(),
            expected_instrument_revision: body.expected_instrument_revision,
            expected_instrument_digest: body.expected_instrument_digest,
            retired_by_admin_user_id: admin_user_id.to_string(),
            reason: body.reason,
            confirmation: COMPUTE_CAPACITY_INSTRUMENT_RETIREMENT_CONFIRMATION.to_string(),
            idempotency_scope: operation_scope("retire", admin_user_id),
            idempotency_key: body.idempotency_key,
        })
        .map_err(classify_store_error)
}

pub(crate) fn adopt_offer_for_admin(
    store: &Store,
    admin_user_id: &str,
    offer_id: &str,
    body: AdoptComputeCapacityInstrumentOfferBody,
) -> Result<ComputeCapacityInstrumentOfferAdoptionWriteReceipt, ComputeCapacityInstrumentServiceError>
{
    if !body.confirm_adoption
        || body.expected_instrument_revision != COMPUTE_CAPACITY_INSTRUMENT_REVISION
    {
        return Err(invalid("采用容量工具前必须确认冻结的工具版本"));
    }
    validate_expected_identity(
        &body.instrument_id,
        body.expected_instrument_revision,
        &body.expected_instrument_digest,
    )?;
    validate_text(offer_id, "Offer ID", 1, 200)?;
    validate_text(&body.expected_publication_id, "发布回执 ID", 1, 200)?;
    validate_digest(&body.expected_offer_digest, "Offer 摘要")?;
    validate_digest(&body.expected_publication_digest, "发布回执摘要")?;
    validate_text(&body.idempotency_key, "幂等键", 1, 200)?;
    if !(1..=MAX_SAFE_INTEGER).contains(&body.expected_offer_version) {
        return Err(invalid("Offer version 必须是 JSON 安全正整数"));
    }
    require_instrument(store, &body.instrument_id)?;
    store
        .adopt_compute_capacity_instrument_offer(AdoptComputeCapacityInstrumentOffer {
            instrument_id: body.instrument_id,
            expected_instrument_revision: body.expected_instrument_revision,
            expected_instrument_digest: body.expected_instrument_digest,
            offer_id: offer_id.to_string(),
            expected_offer_version: body.expected_offer_version,
            expected_offer_digest: body.expected_offer_digest,
            expected_publication_id: body.expected_publication_id,
            expected_publication_digest: body.expected_publication_digest,
            adopted_by_admin_user_id: admin_user_id.to_string(),
            confirmation: COMPUTE_CAPACITY_INSTRUMENT_OFFER_ADOPTION_CONFIRMATION.to_string(),
            idempotency_scope: operation_scope("adopt-offer", admin_user_id),
            idempotency_key: body.idempotency_key,
        })
        .map_err(classify_store_error)
}

pub(crate) fn get_for_admin(
    store: &Store,
    instrument_id: &str,
) -> Result<
    crate::compute_federation::capacity_instrument::ComputeCapacityInstrument,
    ComputeCapacityInstrumentServiceError,
> {
    validate_text(instrument_id, "容量工具 ID", 1, 160)?;
    store
        .compute_capacity_instrument(instrument_id)
        .map_err(classify_store_error)?
        .ok_or(ComputeCapacityInstrumentServiceError::NotFound)
}

pub(crate) fn list_for_admin(
    store: &Store,
    limit: usize,
) -> Result<Vec<ComputeCapacityInstrumentCurrentnessReceipt>, ComputeCapacityInstrumentServiceError>
{
    if !(1..=100).contains(&limit) {
        return Err(invalid("容量工具列表 limit 必须在 1 到 100 之间"));
    }
    store
        .list_compute_capacity_instruments(limit)
        .map_err(classify_store_error)
}

pub(crate) fn currentness_for_admin(
    store: &Store,
    instrument_id: &str,
) -> Result<ComputeCapacityInstrumentCurrentnessReceipt, ComputeCapacityInstrumentServiceError> {
    validate_text(instrument_id, "容量工具 ID", 1, 160)?;
    store
        .compute_capacity_instrument_currentness(instrument_id)
        .map_err(classify_store_error)?
        .ok_or(ComputeCapacityInstrumentServiceError::NotFound)
}

pub(crate) fn offer_adoption_for_admin(
    store: &Store,
    offer_id: &str,
) -> Result<
    crate::compute_federation::capacity_instrument::ComputeCapacityInstrumentOfferAdoptionReceipt,
    ComputeCapacityInstrumentServiceError,
> {
    validate_text(offer_id, "Offer ID", 1, 200)?;
    store
        .compute_capacity_instrument_offer_adoption(offer_id)
        .map_err(classify_store_error)?
        .ok_or(ComputeCapacityInstrumentServiceError::NotFound)
}

fn require_instrument(
    store: &Store,
    instrument_id: &str,
) -> Result<(), ComputeCapacityInstrumentServiceError> {
    get_for_admin(store, instrument_id).map(|_| ())
}

fn classify_store_error(error: AnyError) -> ComputeCapacityInstrumentServiceError {
    let text = format!("{error:#}");
    if text.contains("not found") || text.contains("was not found") || text.contains("is absent") {
        ComputeCapacityInstrumentServiceError::NotFound
    } else {
        ComputeCapacityInstrumentServiceError::Conflict(error)
    }
}

fn operation_scope(operation: &str, admin_user_id: &str) -> String {
    format!("capacity-instrument:{operation}:{admin_user_id}")
}

fn validate_register_body(
    body: &RegisterComputeCapacityInstrumentBody,
) -> Result<(), ComputeCapacityInstrumentServiceError> {
    for (value, label, minimum, maximum) in [
        (&body.instrument_id, "容量工具 ID", 1, 160),
        (&body.sku_id, "SKU ID", 1, 160),
        (&body.availability_sla_tier, "可用率 SLA 等级", 1, 200),
        (&body.region_or_data_zone, "区域或数据区", 1, 200),
        (&body.verification_tier, "验证等级", 1, 200),
        (&body.idempotency_key, "幂等键", 1, 200),
    ] {
        validate_text(value, label, minimum, maximum)?;
    }
    validate_digest(&body.sku_digest, "SKU 摘要")?;
    if body.contract_units.is_empty() || body.contract_units.len() > 64 {
        return Err(invalid("合约单位必须包含 1 到 64 个 meter"));
    }
    let mut previous = None;
    for unit in &body.contract_units {
        validate_text(&unit.meter, "合约 meter", 1, 160)?;
        if !(1..=MAX_SAFE_INTEGER).contains(&unit.unit_size)
            || !(1..=MAX_SAFE_INTEGER).contains(&unit.quantity_units)
            || unit.quantity_units % unit.unit_size != 0
            || previous.is_some_and(|value: &str| value >= unit.meter.as_str())
        {
            return Err(invalid("合约单位必须按 meter 唯一排序且数量为正整倍数"));
        }
        previous = Some(unit.meter.as_str());
    }
    validate_text(
        &body.delivery_window.binding.window_id,
        "交付窗口 ID",
        1,
        160,
    )?;
    validate_digest(&body.delivery_window.binding.window_digest, "交付窗口摘要")?;
    validate_utc_nanos(&body.delivery_window.starts_at_utc, "交付窗口开始")?;
    validate_utc_nanos(&body.delivery_window.ends_at_utc, "交付窗口结束")?;
    let starts = chrono::DateTime::parse_from_rfc3339(&body.delivery_window.starts_at_utc)
        .map_err(|error| invalid_with("交付窗口开始无效", error))?;
    let ends = chrono::DateTime::parse_from_rfc3339(&body.delivery_window.ends_at_utc)
        .map_err(|error| invalid_with("交付窗口结束无效", error))?;
    if starts >= ends {
        return Err(invalid("交付窗口必须是正的半开区间"));
    }
    Ok(())
}

fn validate_expected_identity(
    id: &str,
    revision: i64,
    digest: &str,
) -> Result<(), ComputeCapacityInstrumentServiceError> {
    validate_text(id, "容量工具 ID", 1, 160)?;
    validate_digest(digest, "容量工具摘要")?;
    if revision != COMPUTE_CAPACITY_INSTRUMENT_REVISION {
        return Err(invalid("容量工具 revision 不是冻结版本"));
    }
    Ok(())
}

fn validate_text(
    value: &str,
    label: &'static str,
    minimum: usize,
    maximum: usize,
) -> Result<(), ComputeCapacityInstrumentServiceError> {
    let count = value.chars().count();
    if value.trim() != value
        || !(minimum..=maximum).contains(&count)
        || value.chars().any(char::is_control)
    {
        return Err(invalid(label));
    }
    Ok(())
}

fn validate_digest(
    value: &str,
    label: &'static str,
) -> Result<(), ComputeCapacityInstrumentServiceError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid(label));
    }
    Ok(())
}

fn validate_utc_nanos(
    value: &str,
    label: &'static str,
) -> Result<(), ComputeCapacityInstrumentServiceError> {
    let parsed =
        chrono::DateTime::parse_from_rfc3339(value).map_err(|error| invalid_with(label, error))?;
    if parsed.offset().local_minus_utc() != 0
        || parsed.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true) != value
    {
        return Err(invalid(label));
    }
    Ok(())
}

fn invalid_with(
    message: &'static str,
    source: impl Into<AnyError>,
) -> ComputeCapacityInstrumentServiceError {
    ComputeCapacityInstrumentServiceError::Invalid(source.into().context(message))
}

fn invalid(message: &'static str) -> ComputeCapacityInstrumentServiceError {
    ComputeCapacityInstrumentServiceError::Invalid(anyhow::anyhow!(message))
}
