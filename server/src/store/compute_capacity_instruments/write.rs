use anyhow::{anyhow, bail, Result};
use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, TransactionBehavior};

use crate::compute_federation::capacity_instrument::{
    validate_compute_capacity_instrument, validate_compute_capacity_instrument_activation_receipt,
    validate_compute_capacity_instrument_retirement_receipt, ComputeCapacityInstrument,
    ComputeCapacityInstrumentActivationReceipt, ComputeCapacityInstrumentRetirementReceipt,
    COMPUTE_CAPACITY_INSTRUMENT_ACTIVATION_CONFIRMATION,
    COMPUTE_CAPACITY_INSTRUMENT_ACTIVATION_RECEIPT_SCHEMA,
    COMPUTE_CAPACITY_INSTRUMENT_CANONICALIZATION, COMPUTE_CAPACITY_INSTRUMENT_DIGEST_ALGORITHM,
    COMPUTE_CAPACITY_INSTRUMENT_REGISTRATION_CONFIRMATION,
    COMPUTE_CAPACITY_INSTRUMENT_RETIREMENT_CONFIRMATION,
    COMPUTE_CAPACITY_INSTRUMENT_RETIREMENT_RECEIPT_SCHEMA, COMPUTE_CAPACITY_INSTRUMENT_REVISION,
    COMPUTE_CAPACITY_INSTRUMENT_SCHEMA, COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_CURRENCY,
    COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_UNIT,
};

use super::{
    canonical::{canonical_activation, canonical_instrument, canonical_retirement},
    read::{
        activation_by_idempotency_on, activation_by_instrument_on, canonical_json,
        instrument_by_id_on, instrument_by_idempotency_on, retirement_by_idempotency_on,
        retirement_by_instrument_on,
    },
    types::{
        ActivateComputeCapacityInstrument, ComputeCapacityInstrumentActivationWriteReceipt,
        ComputeCapacityInstrumentRegistrationWriteReceipt,
        ComputeCapacityInstrumentRetirementWriteReceipt, RegisterComputeCapacityInstrument,
        RetireComputeCapacityInstrument, StoredActivation, StoredInstrument, StoredRetirement,
    },
    validation::{validate_digest, validate_exact, validate_expected_identity},
};
use crate::store::{new_id, Store};

impl Store {
    pub(crate) fn register_compute_capacity_instrument(
        &self,
        input: RegisterComputeCapacityInstrument,
    ) -> Result<ComputeCapacityInstrumentRegistrationWriteReceipt> {
        validate_registration_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) =
            instrument_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            validate_registration_replay(&input, &existing)?;
            tx.commit()?;
            return Ok(ComputeCapacityInstrumentRegistrationWriteReceipt {
                instrument: existing.instrument,
                replayed: true,
            });
        }
        if instrument_by_id_on(&tx, &input.instrument_id)?.is_some() {
            bail!("capacity instrument ID already binds another immutable contract");
        }
        let occurred_at = now();
        let mut instrument = ComputeCapacityInstrument {
            schema: COMPUTE_CAPACITY_INSTRUMENT_SCHEMA.to_string(),
            instrument_id: input.instrument_id,
            instrument_revision: COMPUTE_CAPACITY_INSTRUMENT_REVISION,
            instrument_digest: String::new(),
            canonicalization: COMPUTE_CAPACITY_INSTRUMENT_CANONICALIZATION.to_string(),
            digest_algorithm: COMPUTE_CAPACITY_INSTRUMENT_DIGEST_ALGORITHM.to_string(),
            sku_id: input.sku_id,
            sku_digest: input.sku_digest,
            delivery_window: input.delivery_window,
            contract_units: input.contract_units,
            availability_sla_tier: input.availability_sla_tier,
            region_or_data_zone: input.region_or_data_zone,
            verification_tier: input.verification_tier,
            settlement_currency: input.settlement_currency,
            settlement_unit: input.settlement_unit,
            registered_by_admin_user_id: input.registered_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            registered_at: occurred_at.clone(),
            recorded_at: occurred_at,
        };
        instrument.instrument_digest = canonical_instrument(&instrument)?.1;
        validate_compute_capacity_instrument(&instrument)?;
        let (instrument_json, digest) = canonical_instrument(&instrument)?;
        if digest != instrument.instrument_digest {
            bail!("capacity instrument canonical digest drifted before persistence");
        }
        insert_instrument(&tx, &instrument, &instrument_json)?;
        let stored = instrument_by_id_on(&tx, &instrument.instrument_id)?
            .ok_or_else(|| anyhow!("capacity instrument disappeared after insert"))?;
        if stored.instrument != instrument || stored.instrument_json != instrument_json {
            bail!("capacity instrument write failed exact readback");
        }
        tx.commit()?;
        Ok(ComputeCapacityInstrumentRegistrationWriteReceipt {
            instrument,
            replayed: false,
        })
    }

    pub(crate) fn activate_compute_capacity_instrument(
        &self,
        input: ActivateComputeCapacityInstrument,
    ) -> Result<ComputeCapacityInstrumentActivationWriteReceipt> {
        validate_activation_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) =
            activation_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            let instrument = exact_root(
                &tx,
                &input.instrument_id,
                input.expected_instrument_revision,
                &input.expected_instrument_digest,
            )?;
            validate_activation_replay(&input, &existing)?;
            tx.commit()?;
            return Ok(ComputeCapacityInstrumentActivationWriteReceipt {
                instrument,
                activation: existing.activation,
                replayed: true,
            });
        }
        let instrument = exact_root(
            &tx,
            &input.instrument_id,
            input.expected_instrument_revision,
            &input.expected_instrument_digest,
        )?;
        if activation_by_instrument_on(&tx, &input.instrument_id)?.is_some() {
            bail!("capacity instrument is already activated under another idempotency key");
        }
        if input.activated_by_admin_user_id == instrument.registered_by_admin_user_id {
            bail!("capacity instrument activation requires an actor distinct from registrar");
        }
        let occurred_at = now();
        let mut activation = ComputeCapacityInstrumentActivationReceipt {
            schema: COMPUTE_CAPACITY_INSTRUMENT_ACTIVATION_RECEIPT_SCHEMA.to_string(),
            activation_receipt_id: new_id("compute_capacity_instrument_activation"),
            activation_receipt_digest: String::new(),
            canonicalization: COMPUTE_CAPACITY_INSTRUMENT_CANONICALIZATION.to_string(),
            digest_algorithm: COMPUTE_CAPACITY_INSTRUMENT_DIGEST_ALGORITHM.to_string(),
            instrument_id: instrument.instrument_id.clone(),
            instrument_revision: instrument.instrument_revision,
            instrument_digest: instrument.instrument_digest.clone(),
            activated_by_admin_user_id: input.activated_by_admin_user_id,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            activated_at: occurred_at.clone(),
            recorded_at: occurred_at,
        };
        activation.activation_receipt_digest = canonical_activation(&activation)?.1;
        validate_compute_capacity_instrument_activation_receipt(&activation)?;
        let (activation_json, digest) = canonical_activation(&activation)?;
        if digest != activation.activation_receipt_digest {
            bail!("capacity instrument activation digest drifted before persistence");
        }
        insert_activation(&tx, &activation, &activation_json)?;
        let stored = activation_by_instrument_on(&tx, &activation.instrument_id)?
            .ok_or_else(|| anyhow!("capacity instrument activation disappeared after insert"))?;
        if stored.activation != activation || stored.activation_json != activation_json {
            bail!("capacity instrument activation write failed exact readback");
        }
        tx.commit()?;
        Ok(ComputeCapacityInstrumentActivationWriteReceipt {
            instrument,
            activation,
            replayed: false,
        })
    }

    pub(crate) fn retire_compute_capacity_instrument(
        &self,
        input: RetireComputeCapacityInstrument,
    ) -> Result<ComputeCapacityInstrumentRetirementWriteReceipt> {
        validate_retirement_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) =
            retirement_by_idempotency_on(&tx, &input.idempotency_scope, &input.idempotency_key)?
        {
            let instrument = exact_root(
                &tx,
                &input.instrument_id,
                input.expected_instrument_revision,
                &input.expected_instrument_digest,
            )?;
            validate_retirement_replay(&input, &existing)?;
            tx.commit()?;
            return Ok(ComputeCapacityInstrumentRetirementWriteReceipt {
                instrument,
                retirement: existing.retirement,
                replayed: true,
            });
        }
        let instrument = exact_root(
            &tx,
            &input.instrument_id,
            input.expected_instrument_revision,
            &input.expected_instrument_digest,
        )?;
        if retirement_by_instrument_on(&tx, &input.instrument_id)?.is_some() {
            bail!("capacity instrument is already retired under another idempotency key");
        }
        if activation_by_instrument_on(&tx, &input.instrument_id)?.is_none() {
            bail!("only an active capacity instrument can be retired");
        }
        if input.retired_by_admin_user_id == instrument.registered_by_admin_user_id {
            bail!("capacity instrument retirement requires an actor distinct from registrar");
        }
        let occurred_at = now();
        let mut retirement = ComputeCapacityInstrumentRetirementReceipt {
            schema: COMPUTE_CAPACITY_INSTRUMENT_RETIREMENT_RECEIPT_SCHEMA.to_string(),
            retirement_receipt_id: new_id("compute_capacity_instrument_retirement"),
            retirement_receipt_digest: String::new(),
            canonicalization: COMPUTE_CAPACITY_INSTRUMENT_CANONICALIZATION.to_string(),
            digest_algorithm: COMPUTE_CAPACITY_INSTRUMENT_DIGEST_ALGORITHM.to_string(),
            instrument_id: instrument.instrument_id.clone(),
            instrument_revision: instrument.instrument_revision,
            instrument_digest: instrument.instrument_digest.clone(),
            retired_by_admin_user_id: input.retired_by_admin_user_id,
            reason: input.reason,
            confirmation: input.confirmation,
            idempotency_scope: input.idempotency_scope,
            idempotency_key: input.idempotency_key,
            retired_at: occurred_at.clone(),
            recorded_at: occurred_at,
        };
        retirement.retirement_receipt_digest = canonical_retirement(&retirement)?.1;
        validate_compute_capacity_instrument_retirement_receipt(&retirement)?;
        let (retirement_json, digest) = canonical_retirement(&retirement)?;
        if digest != retirement.retirement_receipt_digest {
            bail!("capacity instrument retirement digest drifted before persistence");
        }
        insert_retirement(&tx, &retirement, &retirement_json)?;
        let stored = retirement_by_instrument_on(&tx, &retirement.instrument_id)?
            .ok_or_else(|| anyhow!("capacity instrument retirement disappeared after insert"))?;
        if stored.retirement != retirement || stored.retirement_json != retirement_json {
            bail!("capacity instrument retirement write failed exact readback");
        }
        tx.commit()?;
        Ok(ComputeCapacityInstrumentRetirementWriteReceipt {
            instrument,
            retirement,
            replayed: false,
        })
    }
}

pub(super) fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}

fn exact_root(
    conn: &Connection,
    id: &str,
    revision: i64,
    digest: &str,
) -> Result<ComputeCapacityInstrument> {
    let root = instrument_by_id_on(conn, id)?
        .ok_or_else(|| anyhow!("capacity instrument does not exist"))?;
    if root.instrument.instrument_revision != revision
        || root.instrument.instrument_digest != digest
    {
        bail!("capacity instrument expected revision or digest is stale");
    }
    Ok(root.instrument)
}

fn validate_registration_input(input: &RegisterComputeCapacityInstrument) -> Result<()> {
    for (value, label, maximum) in [
        (&input.instrument_id, "capacity instrument ID", 200),
        (&input.sku_id, "SKU ID", 200),
        (
            &input.registered_by_admin_user_id,
            "registration actor",
            200,
        ),
        (&input.idempotency_scope, "idempotency scope", 200),
        (&input.idempotency_key, "idempotency key", 200),
    ] {
        validate_exact(value, label, maximum)?;
    }
    validate_digest(&input.sku_digest, "SKU digest")?;
    if input.confirmation != COMPUTE_CAPACITY_INSTRUMENT_REGISTRATION_CONFIRMATION
        || input.settlement_currency != COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_CURRENCY
        || input.settlement_unit != COMPUTE_CAPACITY_INSTRUMENT_SETTLEMENT_UNIT
    {
        bail!("capacity instrument registration confirmation or settlement contract is not exact");
    }
    Ok(())
}

fn validate_activation_input(input: &ActivateComputeCapacityInstrument) -> Result<()> {
    validate_expected_identity(
        &input.instrument_id,
        input.expected_instrument_revision,
        &input.expected_instrument_digest,
    )?;
    for (value, label, maximum) in [
        (&input.activated_by_admin_user_id, "activation actor", 200),
        (&input.idempotency_scope, "idempotency scope", 200),
        (&input.idempotency_key, "idempotency key", 200),
    ] {
        validate_exact(value, label, maximum)?;
    }
    if input.confirmation != COMPUTE_CAPACITY_INSTRUMENT_ACTIVATION_CONFIRMATION {
        bail!("capacity instrument activation confirmation is not exact");
    }
    Ok(())
}

fn validate_retirement_input(input: &RetireComputeCapacityInstrument) -> Result<()> {
    validate_expected_identity(
        &input.instrument_id,
        input.expected_instrument_revision,
        &input.expected_instrument_digest,
    )?;
    for (value, label, maximum) in [
        (&input.retired_by_admin_user_id, "retirement actor", 200),
        (&input.idempotency_scope, "idempotency scope", 200),
        (&input.idempotency_key, "idempotency key", 200),
    ] {
        validate_exact(value, label, maximum)?;
    }
    if input.reason.trim() != input.reason
        || !(8..=2_000).contains(&input.reason.chars().count())
        || input.reason.chars().any(char::is_control)
        || input.confirmation != COMPUTE_CAPACITY_INSTRUMENT_RETIREMENT_CONFIRMATION
    {
        bail!("capacity instrument retirement reason or confirmation is invalid");
    }
    Ok(())
}

fn validate_registration_replay(
    input: &RegisterComputeCapacityInstrument,
    stored: &StoredInstrument,
) -> Result<()> {
    let value = &stored.instrument;
    if value.instrument_id != input.instrument_id
        || value.sku_id != input.sku_id
        || value.sku_digest != input.sku_digest
        || value.delivery_window != input.delivery_window
        || value.contract_units != input.contract_units
        || value.availability_sla_tier != input.availability_sla_tier
        || value.region_or_data_zone != input.region_or_data_zone
        || value.verification_tier != input.verification_tier
        || value.settlement_currency != input.settlement_currency
        || value.settlement_unit != input.settlement_unit
        || value.registered_by_admin_user_id != input.registered_by_admin_user_id
        || value.confirmation != input.confirmation
    {
        bail!("capacity instrument registration idempotency key binds different input");
    }
    Ok(())
}

fn validate_activation_replay(
    input: &ActivateComputeCapacityInstrument,
    stored: &StoredActivation,
) -> Result<()> {
    let value = &stored.activation;
    if value.instrument_id != input.instrument_id
        || value.instrument_revision != input.expected_instrument_revision
        || value.instrument_digest != input.expected_instrument_digest
        || value.activated_by_admin_user_id != input.activated_by_admin_user_id
        || value.confirmation != input.confirmation
    {
        bail!("capacity instrument activation idempotency key binds different input");
    }
    Ok(())
}

fn validate_retirement_replay(
    input: &RetireComputeCapacityInstrument,
    stored: &StoredRetirement,
) -> Result<()> {
    let value = &stored.retirement;
    if value.instrument_id != input.instrument_id
        || value.instrument_revision != input.expected_instrument_revision
        || value.instrument_digest != input.expected_instrument_digest
        || value.retired_by_admin_user_id != input.retired_by_admin_user_id
        || value.reason != input.reason
        || value.confirmation != input.confirmation
    {
        bail!("capacity instrument retirement idempotency key binds different input");
    }
    Ok(())
}

fn insert_instrument(
    conn: &Connection,
    value: &ComputeCapacityInstrument,
    json: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO compute_capacity_instruments (
        instrument_id,instrument_schema,instrument_revision,instrument_digest,instrument_json,
        canonicalization,digest_algorithm,sku_id,sku_digest,delivery_window_id,
        delivery_window_digest,delivery_window_starts_at,delivery_window_ends_at,
        contract_units_json,availability_sla_tier,region_or_data_zone,verification_tier,
        settlement_currency,settlement_unit,registered_by_admin_user_id,confirmation,
        idempotency_scope,idempotency_key,registered_at,recorded_at)
        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,
                ?19,?20,?21,?22,?23,?24,?25)",
        params![
            value.instrument_id,
            value.schema,
            value.instrument_revision,
            value.instrument_digest,
            json,
            value.canonicalization,
            value.digest_algorithm,
            value.sku_id,
            value.sku_digest,
            value.delivery_window.binding.window_id,
            value.delivery_window.binding.window_digest,
            value.delivery_window.starts_at_utc,
            value.delivery_window.ends_at_utc,
            canonical_json(&value.contract_units)?,
            value.availability_sla_tier,
            value.region_or_data_zone,
            value.verification_tier,
            value.settlement_currency,
            value.settlement_unit,
            value.registered_by_admin_user_id,
            value.confirmation,
            value.idempotency_scope,
            value.idempotency_key,
            value.registered_at,
            value.recorded_at
        ],
    )?;
    Ok(())
}

fn insert_activation(
    conn: &Connection,
    value: &ComputeCapacityInstrumentActivationReceipt,
    json: &str,
) -> Result<()> {
    conn.execute("INSERT INTO compute_capacity_instrument_activations (
        activation_receipt_id,activation_schema,activation_receipt_digest,activation_receipt_json,
        canonicalization,digest_algorithm,instrument_id,instrument_revision,instrument_digest,
        activated_by_admin_user_id,confirmation,idempotency_scope,idempotency_key,activated_at,recorded_at)
        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)", params![
        value.activation_receipt_id,value.schema,value.activation_receipt_digest,json,
        value.canonicalization,value.digest_algorithm,value.instrument_id,value.instrument_revision,
        value.instrument_digest,value.activated_by_admin_user_id,value.confirmation,
        value.idempotency_scope,value.idempotency_key,value.activated_at,value.recorded_at])?;
    Ok(())
}

fn insert_retirement(
    conn: &Connection,
    value: &ComputeCapacityInstrumentRetirementReceipt,
    json: &str,
) -> Result<()> {
    conn.execute("INSERT INTO compute_capacity_instrument_retirements (
        retirement_receipt_id,retirement_schema,retirement_receipt_digest,retirement_receipt_json,
        canonicalization,digest_algorithm,instrument_id,instrument_revision,instrument_digest,
        retired_by_admin_user_id,reason,confirmation,idempotency_scope,idempotency_key,retired_at,recorded_at)
        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)", params![
        value.retirement_receipt_id,value.schema,value.retirement_receipt_digest,json,
        value.canonicalization,value.digest_algorithm,value.instrument_id,value.instrument_revision,
        value.instrument_digest,value.retired_by_admin_user_id,value.reason,value.confirmation,
        value.idempotency_scope,value.idempotency_key,value.retired_at,value.recorded_at])?;
    Ok(())
}
