use super::model::{ReferenceQuote, ReferenceValuation};

const SCALE: i128 = 1_000_000;
const MAX_QUOTE_LIFETIME_MS: i64 = 300_000;

pub(crate) fn natural(value: &str) -> Result<i64, &'static str> {
    if value.is_empty()
        || value.len() > 19
        || !value.bytes().all(|v| v.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err("invalid_integer");
    }
    value.parse().map_err(|_| "integer_overflow")
}

impl ReferenceQuote {
    pub(crate) fn validate(&self, at_ms: i64) -> Result<(), &'static str> {
        if self.quote_id.is_empty()
            || self.quote_id.len() > 96
            || !self
                .quote_id
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || b"-_.:".contains(&v))
            || self.source.trim().is_empty()
            || self.source.len() > 160
            || self.source.chars().any(char::is_control)
            || self.observed_at_ms <= 0
            || self.observed_at_ms > at_ms
            || self.valid_until_ms <= at_ms
            || self.valid_until_ms <= self.observed_at_ms
            || self.valid_until_ms - self.observed_at_ms > MAX_QUOTE_LIFETIME_MS
        {
            return Err("quote_unavailable");
        }
        for value in [&self.usdt_per_esk_base_units, &self.cny_per_usdt_base_units] {
            if natural(value)? == 0 {
                return Err("quote_unavailable");
            }
        }
        Ok(())
    }

    pub(crate) fn value(
        &self,
        esk_base_units: i64,
        at_ms: i64,
    ) -> Result<ReferenceValuation, &'static str> {
        self.validate(at_ms)?;
        if esk_base_units < 0 {
            return Err("invalid_amount");
        }
        let usdt_rate = natural(&self.usdt_per_esk_base_units)? as i128;
        let cny_rate = natural(&self.cny_per_usdt_base_units)? as i128;
        let numerator = (esk_base_units as i128)
            .checked_mul(usdt_rate)
            .ok_or("valuation_overflow")?;
        let usdt = rounded(numerator, SCALE)?;
        // Round each final estimate once; don't feed rounded USDT into CNY.
        let cny = rounded(
            numerator
                .checked_mul(cny_rate)
                .ok_or("valuation_overflow")?,
            SCALE * SCALE,
        )?;
        Ok(ReferenceValuation {
            usdt_base_units: usdt.to_string(),
            cny_base_units: cny.to_string(),
        })
    }
}

fn rounded(numerator: i128, denominator: i128) -> Result<i64, &'static str> {
    let result = numerator
        .checked_add(denominator / 2)
        .ok_or("valuation_overflow")?
        / denominator;
    i64::try_from(result).map_err(|_| "valuation_overflow")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quote() -> ReferenceQuote {
        ReferenceQuote {
            quote_id: "platform-1".into(),
            source: "Platform reference".into(),
            observed_at_ms: 1_000,
            valid_until_ms: 61_000,
            usdt_per_esk_base_units: "1000000".into(),
            cny_per_usdt_base_units: "7300000".into(),
        }
    }

    #[test]
    fn exact_estimates_do_not_change_esk_or_claim_a_peg() {
        let value = quote().value(80_000_000, 2_000).unwrap();
        assert_eq!(value.usdt_base_units, "80000000");
        assert_eq!(value.cny_base_units, "584000000");
        let mut q = quote();
        q.usdt_per_esk_base_units = "1250000".into();
        assert_eq!(
            q.value(80_000_000, 2_000).unwrap().usdt_base_units,
            "100000000"
        );
    }

    #[test]
    fn freshness_and_invalid_rates_fail_closed() {
        for at in [999, 61_000, i64::MAX] {
            assert!(quote().value(1, at).is_err());
        }
        for rate in ["0", "-1", "01", "1.0", "1e6", "9223372036854775808"] {
            let mut q = quote();
            q.usdt_per_esk_base_units = rate.into();
            assert!(q.value(1, 2_000).is_err());
        }
        let mut q = quote();
        q.valid_until_ms = 301_001;
        assert!(q.validate(2_000).is_err());
    }

    #[test]
    fn rounding_is_once_per_currency_and_overflow_is_rejected() {
        let mut q = quote();
        q.usdt_per_esk_base_units = "500000".into();
        q.cny_per_usdt_base_units = "1000000".into();
        assert_eq!(q.value(1, 2_000).unwrap().cny_base_units, "1");
        assert!(q.value(-1, 2_000).is_err());
        q.usdt_per_esk_base_units = i64::MAX.to_string();
        q.cny_per_usdt_base_units = i64::MAX.to_string();
        assert!(q.value(i64::MAX, 2_000).is_err());
    }
}
