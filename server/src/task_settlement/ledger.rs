use anyhow::{bail, Result};

#[derive(Debug, Clone)]
pub(crate) struct LedgerPosting {
    pub account_key: String,
    pub user_id: Option<String>,
    pub side: &'static str,
    pub amount_micros: i64,
}

pub(super) fn compute_mirror_postings(
    payer_user_id: &str,
    payee_user_id: Option<&str>,
    compute_amount_micros: i64,
    provider_amount_micros: i64,
) -> Result<Vec<LedgerPosting>> {
    if compute_amount_micros < 0 || provider_amount_micros < 0 {
        bail!("影子账本金额不能为负数");
    }
    if provider_amount_micros > compute_amount_micros {
        bail!("节点收益不能高于本次真实计算成本");
    }
    if compute_amount_micros == 0 {
        return Ok(Vec::new());
    }

    let mut postings = vec![LedgerPosting {
        account_key: "consumer.compute_expense".to_string(),
        user_id: Some(payer_user_id.to_string()),
        side: "debit",
        amount_micros: compute_amount_micros,
    }];
    if provider_amount_micros > 0 {
        postings.push(LedgerPosting {
            account_key: "provider.compute_revenue".to_string(),
            user_id: payee_user_id.map(str::to_string),
            side: "credit",
            amount_micros: provider_amount_micros,
        });
    }
    let platform_amount_micros = compute_amount_micros - provider_amount_micros;
    if platform_amount_micros > 0 {
        postings.push(LedgerPosting {
            account_key: "platform.compute_remainder".to_string(),
            user_id: None,
            side: "credit",
            amount_micros: platform_amount_micros,
        });
    }
    ensure_balanced(&postings)?;
    Ok(postings)
}

pub(crate) fn ensure_balanced(postings: &[LedgerPosting]) -> Result<()> {
    let debit = postings
        .iter()
        .filter(|entry| entry.side == "debit")
        .try_fold(0_i64, |sum, entry| sum.checked_add(entry.amount_micros))
        .ok_or_else(|| anyhow::anyhow!("影子账本借方金额溢出"))?;
    let credit = postings
        .iter()
        .filter(|entry| entry.side == "credit")
        .try_fold(0_i64, |sum, entry| sum.checked_add(entry.amount_micros))
        .ok_or_else(|| anyhow::anyhow!("影子账本贷方金额溢出"))?;
    if debit != credit {
        bail!("影子账本借贷不平衡：debit={debit}, credit={credit}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_postings_are_balanced() {
        let postings =
            compute_mirror_postings("consumer", Some("provider"), 100_000, 70_000).unwrap();
        ensure_balanced(&postings).unwrap();
        assert_eq!(postings.len(), 3);
    }

    #[test]
    fn rejects_provider_amount_above_real_cost() {
        assert!(compute_mirror_postings("consumer", Some("provider"), 50, 51).is_err());
    }
}
