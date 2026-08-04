use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_federation::{
    market::{ComputePriceComponent, ComputePriceSnapshot},
    receipts::{ComputeExecutionReceipt, ComputeMeterReading, ComputeSettlementAmounts},
};

pub(super) const MICROS_PER_CNY_FEN: i64 = 10_000;

#[derive(Debug, Clone)]
pub(super) struct ComputedSettlement {
    pub amounts: ComputeSettlementAmounts,
    pub consumer_charge_fen: i64,
    pub verified_usage_digest: String,
    pub compensable_usage_digest: String,
    pub reason_codes: Vec<String>,
}

pub(super) fn calculate_settlement(
    snapshot: &ComputePriceSnapshot,
    execution: &ComputeExecutionReceipt,
    reserved_fen: i64,
) -> Result<ComputedSettlement> {
    if snapshot.currency != "CNY" || !snapshot.fee_rules.is_empty() {
        bail!("v195 仅支持 CNY 且无附加 fee_rules 的基础组件结算");
    }
    let verified = usage_by_meter(&execution.usage.verified_usage)?;
    let compensable = usage_by_meter(&execution.usage.compensable_usage)?;
    let consumer_base = component_total(&snapshot.components, &verified, true)?;
    let provider_payable = component_total(&snapshot.components, &compensable, false)?;
    let consumer_charge_fen = round_micros_to_fen(consumer_base, &snapshot.rounding_mode)?;
    if consumer_charge_fen < 0 || consumer_charge_fen > reserved_fen {
        bail!("结算后的消费者金额超出已冻结预授权");
    }
    let consumer_charge = i128::from(consumer_charge_fen)
        .checked_mul(i128::from(MICROS_PER_CNY_FEN))
        .context("消费者分金额换算为微单位时溢出")?;
    let rounded_max_fen = round_micros_to_fen(
        i128::from(snapshot.consumer_max_amount_micros),
        &snapshot.rounding_mode,
    )?;
    if consumer_charge_fen > rounded_max_fen
        || provider_payable > i128::from(snapshot.provider_max_amount_micros)
        || provider_payable > consumer_charge
    {
        bail!("结算金额超过价格快照上限或无法保持非负平台价差");
    }
    let platform_margin = consumer_charge
        .checked_sub(provider_payable)
        .ok_or_else(|| anyhow!("平台价差下溢"))?;
    let refund_fen = reserved_fen
        .checked_sub(consumer_charge_fen)
        .ok_or_else(|| anyhow!("消费者退款分金额下溢"))?;
    let refund_micros = i128::from(refund_fen)
        .checked_mul(i128::from(MICROS_PER_CNY_FEN))
        .context("消费者退款换算为微单位时溢出")?;

    Ok(ComputedSettlement {
        amounts: ComputeSettlementAmounts {
            consumer_charge_micros: as_i64("消费者结算金额", consumer_charge)?,
            provider_payable_micros: as_i64("Provider 应得金额", provider_payable)?,
            platform_margin_micros: as_i64("平台价差", platform_margin)?,
            third_party_cost_micros: 0,
            transfer_fee_micros: 0,
            storage_fee_micros: 0,
            verification_fee_micros: 0,
            availability_bonus_micros: 0,
            acceptance_bonus_micros: 0,
            delivery_penalty_micros: 0,
            refund_micros: as_i64("消费者退款", refund_micros)?,
        },
        consumer_charge_fen,
        verified_usage_digest: usage_digest("verified_usage", &execution.usage.verified_usage)?,
        compensable_usage_digest: usage_digest(
            "compensable_usage",
            &execution.usage.compensable_usage,
        )?,
        reason_codes: vec![
            "base_components_only_v1".to_string(),
            "consumer_charge_rounded_to_fen".to_string(),
            "provider_credit_pending".to_string(),
        ],
    })
}

fn usage_by_meter(readings: &[ComputeMeterReading]) -> Result<BTreeMap<&str, i64>> {
    let mut result = BTreeMap::new();
    for reading in readings {
        if reading.quantity < 0
            || result
                .insert(reading.meter.as_str(), reading.quantity)
                .is_some()
        {
            bail!("结算用量 meter 重复或数量为负");
        }
    }
    Ok(result)
}

fn component_total(
    components: &[ComputePriceComponent],
    usage: &BTreeMap<&str, i64>,
    consumer: bool,
) -> Result<i128> {
    if components.len() != usage.len() {
        bail!("价格组件与结算用量 meter 集合不一致");
    }
    let mut total = 0_i128;
    for component in components {
        let quantity = *usage
            .get(component.meter.as_str())
            .ok_or_else(|| anyhow!("结算用量缺少价格 meter {}", component.meter))?;
        if component.unit_size <= 0
            || quantity < 0
            || quantity > component.max_units
            || quantity % component.unit_size != 0
            || component.consumer_unit_price_micros < 0
            || component.provider_unit_price_micros < 0
            || component.provider_unit_price_micros > component.consumer_unit_price_micros
        {
            bail!("结算用量或价格不符合价格组件约束");
        }
        let price = if consumer {
            component.consumer_unit_price_micros
        } else {
            component.provider_unit_price_micros
        };
        let amount = i128::from(quantity / component.unit_size)
            .checked_mul(i128::from(price))
            .context("结算价格组件金额溢出")?;
        total = total.checked_add(amount).context("结算组件总金额溢出")?;
    }
    Ok(total)
}

fn round_micros_to_fen(amount: i128, mode: &str) -> Result<i64> {
    if amount < 0 {
        bail!("人民币微单位金额不能为负");
    }
    let divisor = i128::from(MICROS_PER_CNY_FEN);
    let quotient = amount / divisor;
    let remainder = amount % divisor;
    let rounded = match mode {
        "floor" => quotient,
        "ceil" => quotient + i128::from(remainder > 0),
        "half_up" => quotient + i128::from(remainder * 2 >= divisor),
        "half_even" => {
            quotient
                + i128::from(
                    remainder * 2 > divisor || (remainder * 2 == divisor && quotient % 2 != 0),
                )
        }
        _ => bail!("价格快照舍入模式不受 v195 结算支持"),
    };
    as_i64("人民币分金额", rounded)
}

fn usage_digest(label: &str, readings: &[ComputeMeterReading]) -> Result<String> {
    #[derive(Serialize)]
    struct UsageDigest<'a> {
        schema: &'static str,
        usage_kind: &'a str,
        readings: &'a [ComputeMeterReading],
    }
    let payload = UsageDigest {
        schema: "compute_federation.settlement_usage_digest.v1",
        usage_kind: label,
        readings,
    };
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&payload)?)))
}

fn as_i64(label: &str, value: i128) -> Result<i64> {
    i64::try_from(value).with_context(|| format!("{label}超出 i64 范围"))
}
