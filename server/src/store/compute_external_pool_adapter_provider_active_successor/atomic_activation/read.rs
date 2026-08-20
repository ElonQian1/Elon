use anyhow::{bail, ensure, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, types::Type, Connection, OptionalExtension};

use crate::{
    compute_federation::{
        external_pool_adapter_atomic_activation::ExternalPoolAdapterAtomicActivationReceipt,
        provider::PROVIDER_STATUS_ACTIVE,
    },
    store::compute_provider_registry::{
        current_registered_provider_on, registered_provider_version_on,
    },
};

use super::{
    receipt::{audit_stored_receipt, RECEIPT_COLUMNS},
    route_audit::{audit_historical_route, audit_live_route},
    types::{
        HistoricalExternalPoolAdapterAtomicActivationAuthority,
        StoredExternalPoolAdapterAtomicActivation,
    },
};
pub(super) fn receipt_by_id_on(
    connection: &Connection,
    activation_receipt_id: &str,
) -> Result<Option<StoredExternalPoolAdapterAtomicActivation>> {
    receipt_on(
        connection,
        "activation_receipt_id=?1",
        params![activation_receipt_id],
    )
}
fn receipt_by_binding_on(
    connection: &Connection,
    provider_binding_id: &str,
) -> Result<Option<StoredExternalPoolAdapterAtomicActivation>> {
    let (count, receipt_id): (i64, Option<String>) = connection.query_row(
        "SELECT COUNT(*),MIN(activation_receipt_id)
           FROM compute_external_pool_adapter_atomic_activation_receipts
          WHERE provider_binding_id=?1",
        params![provider_binding_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    ensure!(
        count <= 1,
        "V277 provider binding has ambiguous activation history"
    );
    receipt_id
        .map(|receipt_id| {
            receipt_by_id_on(connection, &receipt_id)?
                .ok_or_else(|| anyhow::anyhow!("V277 activation disappeared during binding lookup"))
        })
        .transpose()
}
fn receipt_on<P: rusqlite::Params>(
    connection: &Connection,
    filter: &str,
    values: P,
) -> Result<Option<StoredExternalPoolAdapterAtomicActivation>> {
    connection
        .query_row(
            &format!(
                "SELECT {RECEIPT_COLUMNS} FROM compute_external_pool_adapter_atomic_activation_receipts WHERE {filter}"
            ),
            values,
            |row| {
                let json: String = row.get(3)?;
                let receipt = serde_json::from_str::<ExternalPoolAdapterAtomicActivationReceipt>(&json)
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            Type::Text,
                            Box::new(error),
                        )
                    })?;
                let scalar_values = (0..79)
                    .map(|index| row.get(index))
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(StoredExternalPoolAdapterAtomicActivation {
                    receipt,
                    scalar_values,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}
pub(in crate::store) fn historical_external_pool_adapter_atomic_activation_authority_on(
    connection: &Connection,
    activation_receipt_id: &str,
    expected_activation_receipt_digest: &str,
    expected_activation_root_digest: &str,
) -> Result<Option<HistoricalExternalPoolAdapterAtomicActivationAuthority>> {
    let Some(stored) = receipt_by_id_on(connection, activation_receipt_id)? else {
        return Ok(None);
    };
    if stored.receipt.activation_receipt_digest != expected_activation_receipt_digest
        || stored.receipt.activation.identity.activation_root_digest
            != expected_activation_root_digest
    {
        bail!("V277 historical activation identity is not exact");
    }
    let evidence_checked_at = stored.receipt.activation.evidence_checked_at.clone();
    authority_from_stored(
        connection,
        stored,
        ProjectionAudit::Live(Some(&evidence_checked_at)),
    )
    .map(Some)
}
pub(in crate::store) fn historical_external_pool_adapter_atomic_activation_for_binding_on(
    connection: &Connection,
    provider_binding_id: &str,
    checked_at: &str,
) -> Result<Option<HistoricalExternalPoolAdapterAtomicActivationAuthority>> {
    validate_checked_at(checked_at)?;
    let Some(stored) = receipt_by_binding_on(connection, provider_binding_id)? else {
        return Ok(None);
    };
    authority_from_stored(connection, stored, ProjectionAudit::Live(Some(checked_at))).map(Some)
}
pub(in crate::store) fn historical_external_pool_adapter_atomic_activation_for_observed_provider_on(
    connection: &Connection,
    provider_binding_id: &str,
    observed_provider_policy_revision: i64,
    observed_provider_digest: &str,
) -> Result<Option<HistoricalExternalPoolAdapterAtomicActivationAuthority>> {
    let Some(stored) = receipt_by_binding_on(connection, provider_binding_id)? else {
        return Ok(None);
    };
    authority_from_stored(
        connection,
        stored,
        ProjectionAudit::Historical {
            provider_policy_revision: observed_provider_policy_revision,
            provider_digest: observed_provider_digest,
        },
    )
    .map(Some)
}

#[derive(Clone, Copy)]
enum ProjectionAudit<'a> {
    Live(Option<&'a str>),
    Historical {
        provider_policy_revision: i64,
        provider_digest: &'a str,
    },
}

fn authority_from_stored(
    connection: &Connection,
    stored: StoredExternalPoolAdapterAtomicActivation,
    projection_audit: ProjectionAudit<'_>,
) -> Result<HistoricalExternalPoolAdapterAtomicActivationAuthority> {
    let receipt = audit_stored_receipt(stored, None)?;
    let activation = &receipt.activation;
    let identity = &activation.identity;
    let target = &activation.provider_transition.target_active_provider;
    let genesis_id = connection
        .query_row(
            "SELECT active_successor_receipt_id
               FROM compute_external_pool_adapter_provider_active_successor_receipts
              WHERE activation_witness_id=?1 AND activation_witness_digest=?2
                AND activation_root_digest=?3 AND successor_sequence=1",
            params![
                receipt.activation_receipt_id,
                receipt.activation_receipt_digest,
                identity.activation_root_digest,
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(genesis_id) = genesis_id else {
        bail!("V277 activation lost its exact V274 genesis witness");
    };
    let genesis = super::super::read::receipt_by_id_on(connection, &genesis_id)?
        .ok_or_else(|| anyhow::anyhow!("V277 activation genesis disappeared"))?
        .receipt;
    let successor = &genesis.successor;
    if successor.activation.activation_root_digest != identity.activation_root_digest
        || successor.activation_witness.activation_witness_id != receipt.activation_receipt_id
        || successor.activation_witness.activation_witness_digest
            != receipt.activation_receipt_digest
        || successor.activation_target_updated_at != activation.activation_target_updated_at
        || successor.evidence_checked_at != activation.evidence_checked_at
    {
        bail!("V277/V274 witness or dual-time binding is not exact");
    }
    let (provider, checked_at) = match projection_audit {
        ProjectionAudit::Live(checked_at) => (
            current_registered_provider_on(connection, &target.provider_id)?
                .ok_or_else(|| anyhow::anyhow!("V277 activation lost its live Provider"))?,
            checked_at,
        ),
        ProjectionAudit::Historical {
            provider_policy_revision,
            provider_digest,
        } => {
            let provider = registered_provider_version_on(
                connection,
                &target.provider_id,
                provider_policy_revision,
            )?
            .ok_or_else(|| anyhow::anyhow!("V277 activation lost historical Provider version"))?;
            if provider.provider_digest != provider_digest {
                bail!("V277 historical Provider digest is not exact");
            }
            (provider, None)
        }
    };
    let provider_json = serde_json::to_string(&provider.provider)?;
    let root = &successor.activation.activation_root;
    let source_provider =
        serde_json::from_str::<crate::compute_federation::provider::ComputeProvider>(
            &activation
                .provider_transition
                .source_registering_provider
                .provider_json,
        )?;
    let initial_provider = serde_json::from_str::<
        crate::compute_federation::provider::ComputeProvider,
    >(&target.provider_json)?;
    let adapter = provider.provider.adapter.as_ref();
    let initial_adapter = initial_provider.adapter.as_ref();
    let is_initial_pair = provider.provider.policy_revision == target.provider_policy_revision;
    if provider.provider.status != PROVIDER_STATUS_ACTIVE
        || provider.provider.provider_id != target.provider_id
        || provider.provider.policy_revision < target.provider_policy_revision
        || provider.provider.provider_kind != initial_provider.provider_kind
        || provider.provider.owner_account_id != root.provider_owner_account_id
        || provider.provider.created_at != source_provider.created_at
        || adapter.map(|value| value.adapter_id.as_str())
            != Some(root.route_adapter_projection_id.as_str())
        || adapter.map(|value| value.adapter_version.as_str())
            != initial_adapter.map(|value| value.adapter_version.as_str())
        || adapter.map(|value| value.config_revision)
            != initial_adapter.map(|value| value.config_revision)
        || adapter.map(|value| value.config_digest.as_str())
            != initial_adapter.map(|value| value.config_digest.as_str())
        || (is_initial_pair
            && (provider.provider_digest != target.provider_digest
                || provider_json != target.provider_json))
    {
        bail!("V277 live active Provider pair drifted");
    }
    match projection_audit {
        ProjectionAudit::Live(_) => audit_live_route(connection, &receipt, checked_at)?,
        ProjectionAudit::Historical { .. } => audit_historical_route(connection, &receipt)?,
    }
    Ok(HistoricalExternalPoolAdapterAtomicActivationAuthority::new(
        receipt,
        genesis,
        provider.provider,
    ))
}

fn validate_checked_at(value: &str) -> Result<()> {
    let parsed = DateTime::parse_from_rfc3339(value)?;
    let now = Utc::now();
    ensure!(
        parsed.offset().local_minus_utc() == 0
            && parsed.to_rfc3339_opts(SecondsFormat::Nanos, true) == value
            && parsed.with_timezone(&Utc) >= now - chrono::Duration::minutes(5)
            && parsed.with_timezone(&Utc) <= now + chrono::Duration::minutes(5),
        "V277 active historical lookup checked_at is not current canonical UTC nanos"
    );
    Ok(())
}
