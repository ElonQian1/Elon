use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::capacity_instrument::{
    validate_compute_capacity_instrument, validate_compute_capacity_instrument_activation_receipt,
    validate_compute_capacity_instrument_offer_adoption_receipt,
    validate_compute_capacity_instrument_retirement_receipt,
};

use super::{
    canonical::{
        canonical_activation, canonical_adoption, canonical_instrument, canonical_retirement,
    },
    read::{canonical_json, instrument_by_id_on},
    types::{StoredActivation, StoredAdoption, StoredInstrument, StoredRetirement},
};

pub(super) fn audit_instrument(
    conn: &Connection,
    stored: StoredInstrument,
) -> Result<StoredInstrument> {
    validate_compute_capacity_instrument(&stored.instrument)?;
    let (json, digest) = canonical_instrument(&stored.instrument)?;
    let units_json = canonical_json(&stored.instrument.contract_units)?;
    let window = &stored.instrument.delivery_window;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_capacity_instruments
          WHERE instrument_id=?1 AND instrument_schema=?2 AND instrument_revision=?3
            AND instrument_digest=?4 AND instrument_json=?5 AND canonicalization=?6
            AND digest_algorithm=?7 AND sku_id=?8 AND sku_digest=?9
            AND delivery_window_id=?10 AND delivery_window_digest=?11
            AND delivery_window_starts_at=?12 AND delivery_window_ends_at=?13
            AND contract_units_json=?14 AND availability_sla_tier=?15
            AND region_or_data_zone=?16 AND verification_tier=?17
            AND settlement_currency=?18 AND settlement_unit=?19
            AND registered_by_admin_user_id=?20 AND confirmation=?21
            AND idempotency_scope=?22 AND idempotency_key=?23
            AND registered_at=?24 AND recorded_at=?25",
            params![
                stored.instrument.instrument_id,
                stored.instrument.schema,
                stored.instrument.instrument_revision,
                stored.instrument.instrument_digest,
                stored.instrument_json,
                stored.instrument.canonicalization,
                stored.instrument.digest_algorithm,
                stored.instrument.sku_id,
                stored.instrument.sku_digest,
                window.binding.window_id,
                window.binding.window_digest,
                window.starts_at_utc,
                window.ends_at_utc,
                units_json,
                stored.instrument.availability_sla_tier,
                stored.instrument.region_or_data_zone,
                stored.instrument.verification_tier,
                stored.instrument.settlement_currency,
                stored.instrument.settlement_unit,
                stored.instrument.registered_by_admin_user_id,
                stored.instrument.confirmation,
                stored.instrument.idempotency_scope,
                stored.instrument.idempotency_key,
                stored.instrument.registered_at,
                stored.instrument.recorded_at
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if json != stored.instrument_json || digest != stored.instrument.instrument_digest || !exact {
        bail!("capacity instrument failed exact readback audit");
    }
    Ok(stored)
}

pub(super) fn audit_activation(
    conn: &Connection,
    stored: StoredActivation,
) -> Result<StoredActivation> {
    validate_compute_capacity_instrument_activation_receipt(&stored.activation)?;
    let (json, digest) = canonical_activation(&stored.activation)?;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_capacity_instrument_activations
          WHERE activation_receipt_id=?1 AND activation_schema=?2
            AND activation_receipt_digest=?3 AND activation_receipt_json=?4
            AND canonicalization=?5 AND digest_algorithm=?6 AND instrument_id=?7
            AND instrument_revision=?8 AND instrument_digest=?9
            AND activated_by_admin_user_id=?10 AND confirmation=?11
            AND idempotency_scope=?12 AND idempotency_key=?13
            AND activated_at=?14 AND recorded_at=?15",
            params![
                stored.activation.activation_receipt_id,
                stored.activation.schema,
                stored.activation.activation_receipt_digest,
                stored.activation_json,
                stored.activation.canonicalization,
                stored.activation.digest_algorithm,
                stored.activation.instrument_id,
                stored.activation.instrument_revision,
                stored.activation.instrument_digest,
                stored.activation.activated_by_admin_user_id,
                stored.activation.confirmation,
                stored.activation.idempotency_scope,
                stored.activation.idempotency_key,
                stored.activation.activated_at,
                stored.activation.recorded_at
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    audit_root(
        conn,
        &stored.activation.instrument_id,
        stored.activation.instrument_revision,
        &stored.activation.instrument_digest,
        "activation",
    )?;
    if json != stored.activation_json
        || digest != stored.activation.activation_receipt_digest
        || !exact
    {
        bail!("capacity instrument activation failed exact readback audit");
    }
    Ok(stored)
}

pub(super) fn audit_retirement(
    conn: &Connection,
    stored: StoredRetirement,
) -> Result<StoredRetirement> {
    validate_compute_capacity_instrument_retirement_receipt(&stored.retirement)?;
    let (json, digest) = canonical_retirement(&stored.retirement)?;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_capacity_instrument_retirements
          WHERE retirement_receipt_id=?1 AND retirement_schema=?2
            AND retirement_receipt_digest=?3 AND retirement_receipt_json=?4
            AND canonicalization=?5 AND digest_algorithm=?6 AND instrument_id=?7
            AND instrument_revision=?8 AND instrument_digest=?9
            AND retired_by_admin_user_id=?10 AND reason=?11 AND confirmation=?12
            AND idempotency_scope=?13 AND idempotency_key=?14
            AND retired_at=?15 AND recorded_at=?16",
            params![
                stored.retirement.retirement_receipt_id,
                stored.retirement.schema,
                stored.retirement.retirement_receipt_digest,
                stored.retirement_json,
                stored.retirement.canonicalization,
                stored.retirement.digest_algorithm,
                stored.retirement.instrument_id,
                stored.retirement.instrument_revision,
                stored.retirement.instrument_digest,
                stored.retirement.retired_by_admin_user_id,
                stored.retirement.reason,
                stored.retirement.confirmation,
                stored.retirement.idempotency_scope,
                stored.retirement.idempotency_key,
                stored.retirement.retired_at,
                stored.retirement.recorded_at
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    audit_root(
        conn,
        &stored.retirement.instrument_id,
        stored.retirement.instrument_revision,
        &stored.retirement.instrument_digest,
        "retirement",
    )?;
    if json != stored.retirement_json
        || digest != stored.retirement.retirement_receipt_digest
        || !exact
    {
        bail!("capacity instrument retirement failed exact readback audit");
    }
    Ok(stored)
}

pub(super) fn audit_adoption(conn: &Connection, stored: StoredAdoption) -> Result<StoredAdoption> {
    validate_compute_capacity_instrument_offer_adoption_receipt(&stored.adoption)?;
    let (json, digest) = canonical_adoption(&stored.adoption)?;
    let exact = conn
        .query_row(
            "SELECT 1 FROM compute_capacity_instrument_offer_adoptions
          WHERE adoption_receipt_id=?1 AND adoption_schema=?2
            AND adoption_receipt_digest=?3 AND adoption_receipt_json=?4
            AND canonicalization=?5 AND digest_algorithm=?6 AND instrument_id=?7
            AND instrument_revision=?8 AND instrument_digest=?9 AND offer_id=?10
            AND offer_version=?11 AND offer_digest=?12 AND publication_id=?13
            AND publication_digest=?14 AND adopted_by_admin_user_id=?15
            AND confirmation=?16 AND idempotency_scope=?17 AND idempotency_key=?18
            AND adopted_at=?19 AND recorded_at=?20",
            params![
                stored.adoption.adoption_receipt_id,
                stored.adoption.schema,
                stored.adoption.adoption_receipt_digest,
                stored.adoption_json,
                stored.adoption.canonicalization,
                stored.adoption.digest_algorithm,
                stored.adoption.instrument_id,
                stored.adoption.instrument_revision,
                stored.adoption.instrument_digest,
                stored.adoption.offer_id,
                stored.adoption.offer_version,
                stored.adoption.offer_digest,
                stored.adoption.publication_id,
                stored.adoption.publication_digest,
                stored.adoption.adopted_by_admin_user_id,
                stored.adoption.confirmation,
                stored.adoption.idempotency_scope,
                stored.adoption.idempotency_key,
                stored.adoption.adopted_at,
                stored.adoption.recorded_at
            ],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    audit_root(
        conn,
        &stored.adoption.instrument_id,
        stored.adoption.instrument_revision,
        &stored.adoption.instrument_digest,
        "adoption",
    )?;
    if json != stored.adoption_json || digest != stored.adoption.adoption_receipt_digest || !exact {
        bail!("capacity instrument adoption failed exact readback audit");
    }
    Ok(stored)
}

fn audit_root(
    conn: &Connection,
    instrument_id: &str,
    instrument_revision: i64,
    instrument_digest: &str,
    label: &str,
) -> Result<()> {
    let root = instrument_by_id_on(conn, instrument_id)?
        .with_context(|| format!("capacity instrument {label} lost its root"))?;
    if root.instrument.instrument_revision != instrument_revision
        || root.instrument.instrument_digest != instrument_digest
    {
        bail!("capacity instrument {label} root lineage drifted");
    }
    Ok(())
}
