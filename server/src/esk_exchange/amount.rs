use anyhow::{anyhow, bail, Result};

pub(crate) const ASSET_DECIMALS: usize = 6;
pub(crate) const ASSET_SCALE: i64 = 1_000_000;

pub(crate) fn parse_amount(value: &str, label: &str) -> Result<i64> {
    let value = value.trim();
    if value.is_empty() || value.starts_with('+') || value.starts_with('-') {
        bail!("{label}必须是正数十进制字符串");
    }
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fractional = parts.next();
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
    {
        bail!("{label}格式无效");
    }
    let fractional = fractional.unwrap_or_default();
    if (value.contains('.') && fractional.is_empty())
        || fractional.len() > ASSET_DECIMALS
        || !fractional.bytes().all(|byte| byte.is_ascii_digit())
    {
        bail!("{label}最多支持六位小数");
    }
    let whole = whole
        .parse::<i64>()
        .map_err(|_| anyhow!("{label}超出范围"))?;
    let padded = format!("{fractional:0<width$}", width = ASSET_DECIMALS);
    let fraction = if padded.is_empty() {
        0
    } else {
        padded
            .parse::<i64>()
            .map_err(|_| anyhow!("{label}格式无效"))?
    };
    let units = whole
        .checked_mul(ASSET_SCALE)
        .and_then(|current| current.checked_add(fraction))
        .ok_or_else(|| anyhow!("{label}超出范围"))?;
    if units <= 0 {
        bail!("{label}必须大于 0");
    }
    Ok(units)
}

pub(crate) fn format_amount(units: i64) -> String {
    let sign = if units < 0 { "-" } else { "" };
    let absolute = units.unsigned_abs();
    format!(
        "{sign}{}.{:06}",
        absolute / ASSET_SCALE as u64,
        absolute % ASSET_SCALE as u64
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_six_decimal_codec_rejects_float_ambiguity() {
        assert_eq!(parse_amount("1.000001", "金额").unwrap(), 1_000_001);
        assert_eq!(format_amount(1_000_001), "1.000001");
        for value in ["", "0", "-1", "+1", ".1", "1.", "1.0000001", "1e6"] {
            assert!(parse_amount(value, "金额").is_err(), "accepted {value}");
        }
    }
}
