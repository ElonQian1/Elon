use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, Connection};

use crate::{
    compute_federation::external_pool_adapter_sandbox_reattestation::SANDBOX_REATTESTATION_CURRENTNESS_SCHEMA,
    store::{
        compute_external_pool_adapter_vulnerability_reattestation::external_pool_adapter_vulnerability_reattestation_currentness_on,
        Store,
    },
};

use super::{read::*, types::*};

fn currentness_on(
    conn: &Connection,
    release_id: &str,
    checked_at: &str,
) -> Result<Option<ExternalPoolAdapterSandboxReattestationCurrentness>> {
    let Some(head) = head_by_release_on(conn, release_id)? else {
        return Ok(None);
    };
    let id = &head.receipt.reattestation_receipt_id;
    let digest = &head.receipt.reattestation_receipt_digest;
    let statuses: (String, String, String, String) = conn.query_row(
        "SELECT head_status,registry_release_status,sandbox_verifier_key_status,
                revocation_status
           FROM compute_external_pool_adapter_sandbox_reattestation_current
          WHERE registry_release_id=?1 AND reattestation_receipt_id=?2
            AND reattestation_receipt_digest=?3",
        params![release_id, id, digest],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let revocation = revocation_by_receipt_on(conn, id)?;
    let verified = DateTime::parse_from_rfc3339(&head.receipt.reattestation.verified_at)?;
    let expires =
        DateTime::parse_from_rfc3339(&head.receipt.reattestation.binding.report_expires_at)?;
    let checked = DateTime::parse_from_rfc3339(checked_at)?;
    if checked.offset().local_minus_utc() != 0
        || checked.to_rfc3339_opts(SecondsFormat::Nanos, true) != checked_at
    {
        bail!("sandbox re-attestation checked_at is not canonical UTC nanoseconds");
    }
    let vulnerability_currentness =
        external_pool_adapter_vulnerability_reattestation_currentness_on(
            conn, release_id, checked_at,
        )?;
    let vulnerability_is_exact_current = vulnerability_currentness.as_ref().is_some_and(|item| {
        item.current_status == "verified_current"
            && item.reattestation.reattestation_receipt_id
                == head
                    .receipt
                    .reattestation
                    .binding
                    .vulnerability_reattestation_receipt_id
            && item.reattestation.reattestation_receipt_digest
                == head
                    .receipt
                    .reattestation
                    .binding
                    .vulnerability_reattestation_receipt_digest
    });
    let exact_roots = statuses.0 == "head"
        && statuses.1 == "release_current"
        && vulnerability_is_exact_current
        && statuses.2 == "active"
        && revocation.is_none();
    let expected_current = exact_roots && checked >= verified && checked < expires;
    if statuses.3
        != if revocation.is_none() {
            "none"
        } else {
            "revoked"
        }
    {
        bail!("sandbox re-attestation current view failed exact audit");
    }
    if expected_current {
        current_external_pool_adapter_sandbox_reattestation_authority_on(
            conn, release_id, id, digest, checked_at,
        )?
        .ok_or_else(|| anyhow::anyhow!("current sandbox re-attestation was not found"))?;
    }
    Ok(Some(ExternalPoolAdapterSandboxReattestationCurrentness {
        schema: SANDBOX_REATTESTATION_CURRENTNESS_SCHEMA,
        reattestation: head.summary(),
        revocation: revocation.as_ref().map(|item| item.summary()),
        current_status: if expected_current {
            "verified_current"
        } else {
            "historical_only"
        }
        .into(),
        head_status: statuses.0,
        registry_release_status: statuses.1,
        vulnerability_reattestation_status: if vulnerability_is_exact_current {
            "verified_current"
        } else {
            "historical_only"
        }
        .into(),
        sandbox_verifier_key_status: statuses.2,
        report_validity_status: if checked >= verified && checked < expires {
            "current"
        } else {
            "expired"
        }
        .into(),
        revocation_status: statuses.3,
    }))
}

impl Store {
    pub(crate) fn external_pool_adapter_sandbox_reattestation_currentness(
        &self,
        release_id: &str,
    ) -> Result<Option<ExternalPoolAdapterSandboxReattestationCurrentness>> {
        let conn = self.conn()?;
        let checked_at = Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true);
        currentness_on(&conn, release_id, &checked_at)
    }

    #[cfg(test)]
    pub(crate) fn external_pool_adapter_sandbox_reattestation_currentness_at(
        &self,
        release_id: &str,
        checked_at: &str,
    ) -> Result<Option<ExternalPoolAdapterSandboxReattestationCurrentness>> {
        let conn = self.conn()?;
        currentness_on(&conn, release_id, checked_at)
    }
}
