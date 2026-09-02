use anyhow::{anyhow, bail, Result};

use super::{amount::ASSET_SCALE, EskExchangeDirection};

pub(crate) fn calculate_quote(
    direction: EskExchangeDirection,
    input_units: i64,
    price_units: i64,
    fee_bps: u16,
) -> Result<(i64, i64, i64)> {
    if input_units <= 0 || price_units <= 0 {
        bail!("兑换金额和价格必须大于 0");
    }
    if fee_bps > 1_000 {
        bail!("兑换手续费必须在 0..=1000 基点之间");
    }
    let input = i128::from(input_units);
    let price = i128::from(price_units);
    let scale = i128::from(ASSET_SCALE);
    let gross = match direction {
        EskExchangeDirection::UsdtToEsk => {
            input
                .checked_mul(scale)
                .ok_or_else(|| anyhow!("兑换报价超出范围"))?
                / price
        }
        EskExchangeDirection::EskToUsdt => {
            input
                .checked_mul(price)
                .ok_or_else(|| anyhow!("兑换报价超出范围"))?
                / scale
        }
    };
    if gross <= 0 || gross > i128::from(i64::MAX) {
        bail!("兑换金额过小或超出范围");
    }
    let fee = if fee_bps == 0 {
        0
    } else {
        gross
            .checked_mul(i128::from(fee_bps))
            .and_then(|value| value.checked_add(9_999))
            .ok_or_else(|| anyhow!("兑换手续费超出范围"))?
            / 10_000
    };
    let net = gross
        .checked_sub(fee)
        .ok_or_else(|| anyhow!("兑换净到账金额无效"))?;
    if net <= 0 {
        bail!("兑换金额扣除手续费后必须大于 0");
    }
    Ok((gross as i64, fee as i64, net as i64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_both_directions_with_documented_rounding() {
        assert_eq!(
            calculate_quote(EskExchangeDirection::UsdtToEsk, 1_000_000, 2_000_000, 30).unwrap(),
            (500_000, 1_500, 498_500)
        );
        assert_eq!(
            calculate_quote(EskExchangeDirection::EskToUsdt, 1_000_001, 1_500_000, 1).unwrap(),
            (1_500_001, 151, 1_499_850)
        );
    }

    #[test]
    fn rejects_zero_net_and_invalid_ranges() {
        assert!(calculate_quote(EskExchangeDirection::UsdtToEsk, 1, i64::MAX, 30).is_err());
        assert!(calculate_quote(EskExchangeDirection::EskToUsdt, 1, 1, 1_001).is_err());
    }
}
