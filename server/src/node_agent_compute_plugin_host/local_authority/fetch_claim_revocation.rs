use anyhow::{bail, Context, Result};
use rusqlite::{params, Transaction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FetchClaimRevocationReason {
    AuthorityEpochAdvancedByKeyring,
    AuthorityEpochAdvancedByPlan,
}

impl FetchClaimRevocationReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::AuthorityEpochAdvancedByKeyring => "authority_epoch_advanced_by_keyring",
            Self::AuthorityEpochAdvancedByPlan => "authority_epoch_advanced_by_plan",
        }
    }
}

pub(super) fn revoke_for_authority_epoch_advance(
    transaction: &Transaction<'_>,
    expected_old_epoch: i64,
    expected_new_epoch: i64,
    resolved_at_ms: i64,
    reason: FetchClaimRevocationReason,
) -> Result<()> {
    let calculated_new_epoch = expected_old_epoch
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_EPOCH_EXHAUSTED"))?;
    if expected_old_epoch < 0 || expected_new_epoch != calculated_new_epoch || resolved_at_ms < 0 {
        bail!("COMPUTE_PLUGIN_FETCH_REVOKE_AUTHORITY_EPOCH_INVALID");
    }

    let (authority_epoch, process_owner_epoch, trusted_time_high_water_ms) = transaction
        .query_row(
            r#"SELECT authority_epoch, process_owner_epoch, trusted_time_high_water_ms
            FROM authority_meta WHERE singleton = 1"#,
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                ))
            },
        )
        .context("COMPUTE_PLUGIN_FETCH_REVOKE_AUTHORITY_READ")?;
    if authority_epoch != expected_old_epoch
        || process_owner_epoch < 0
        || trusted_time_high_water_ms.is_some_and(|high_water| resolved_at_ms < high_water)
    {
        bail!("COMPUTE_PLUGIN_FETCH_REVOKE_AUTHORITY_CHANGED");
    }

    let impossible_claims = transaction
        .query_row(
            r#"SELECT COUNT(*) FROM fetch_claims
            WHERE state = 'prepared' AND (
                authority_epoch > ?1
                OR process_owner_epoch > ?2
                OR prepared_at_ms > ?3
            )"#,
            params![expected_old_epoch, process_owner_epoch, resolved_at_ms],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_FETCH_REVOKE_FUTURE_CLAIMS_CHECK")?;
    if impossible_claims != 0 {
        bail!("COMPUTE_PLUGIN_FETCH_REVOKE_FUTURE_CLAIM");
    }

    transaction
        .execute(
            r#"UPDATE fetch_claims SET
                state = 'revoked', resolved_at_ms = ?1, resolution_reason = ?2
            WHERE state = 'prepared' AND authority_epoch <= ?3"#,
            params![resolved_at_ms, reason.as_str(), expected_old_epoch],
        )
        .context("COMPUTE_PLUGIN_FETCH_REVOKE_AUTHORITY_EPOCH")?;
    let remaining = transaction
        .query_row(
            "SELECT COUNT(*) FROM fetch_claims WHERE state = 'prepared'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .context("COMPUTE_PLUGIN_FETCH_REVOKE_REMAINING_CHECK")?;
    if remaining != 0 {
        bail!("COMPUTE_PLUGIN_FETCH_REVOKE_INCOMPLETE");
    }
    Ok(())
}
