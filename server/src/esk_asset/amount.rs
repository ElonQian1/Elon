use anyhow::{anyhow, bail, Result};

use super::model::{ESK_DECIMALS, ESK_SCALE};

pub(crate) fn parse_esk_amount(value: &str) -> Result<i64> {
    let value = value.trim();
    if value.is_empty() || value.starts_with('+') || value.starts_with('-') {
        bail!("ESK 金额必须是正数十进制字符串");
    }
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fractional = parts.next();
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
    {
        bail!("ESK 金额格式无效");
    }
    let fractional = fractional.unwrap_or_default();
    if (value.contains('.') && fractional.is_empty())
        || fractional.len() > ESK_DECIMALS as usize
        || !fractional.bytes().all(|byte| byte.is_ascii_digit())
    {
        bail!("ESK 金额最多支持六位小数");
    }
    let whole = whole
        .parse::<i64>()
        .map_err(|_| anyhow!("ESK 金额超出范围"))?;
    let padded = format!("{fractional:0<width$}", width = ESK_DECIMALS as usize);
    let fraction = if padded.is_empty() {
        0
    } else {
        padded
            .parse::<i64>()
            .map_err(|_| anyhow!("ESK 金额格式无效"))?
    };
    let base_units = whole
        .checked_mul(ESK_SCALE)
        .and_then(|value| value.checked_add(fraction))
        .ok_or_else(|| anyhow!("ESK 金额超出范围"))?;
    if base_units <= 0 {
        bail!("ESK 金额必须大于 0");
    }
    Ok(base_units)
}

pub(crate) fn format_esk_amount(base_units: i64) -> String {
    let sign = if base_units < 0 { "-" } else { "" };
    let absolute = base_units.unsigned_abs();
    let scale = ESK_SCALE as u64;
    format!(
        "{sign}{}.{:0width$}",
        absolute / scale,
        absolute % scale,
        width = ESK_DECIMALS as usize
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats_six_decimal_esk_amounts() {
        assert_eq!(parse_esk_amount("1250.000000").unwrap(), 1_250_000_000);
        assert_eq!(parse_esk_amount("0.000001").unwrap(), 1);
        assert_eq!(format_esk_amount(1_250_000_000), "1250.000000");
        assert_eq!(format_esk_amount(-1), "-0.000001");
    }

    #[test]
    fn rejects_non_positive_or_over_precise_amounts() {
        for value in ["0", "-1", "+1", "1.", "1.0000001"] {
            assert!(parse_esk_amount(value).is_err(), "accepted {value}");
        }
    }
}
