use anyhow::{bail, Context, Result};
use rusqlite::{params, types::Type, Connection, OptionalExtension, Row};

use crate::{
    compute_federation::capacity_instrument::{
        ComputeCapacityInstrument, ComputeCapacityInstrumentOfferAdoptionReceipt,
        COMPUTE_CAPACITY_INSTRUMENT_CURRENTNESS_SCHEMA, COMPUTE_CAPACITY_INSTRUMENT_STATUS_ACTIVE,
        COMPUTE_CAPACITY_INSTRUMENT_STATUS_REGISTERED, COMPUTE_CAPACITY_INSTRUMENT_STATUS_RETIRED,
    },
    store::Store,
};

use super::{
    audit::{audit_activation, audit_adoption, audit_instrument, audit_retirement},
    types::{
        ComputeCapacityInstrumentCurrentnessReceipt, StoredActivation, StoredAdoption,
        StoredInstrument, StoredRetirement,
    },
    validation::validate_exact,
};

pub(super) fn instrument_by_id_on(
    conn: &Connection,
    instrument_id: &str,
) -> Result<Option<StoredInstrument>> {
    instrument_on(conn, "WHERE instrument_id=?1", params![instrument_id])
}

pub(super) fn instrument_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredInstrument>> {
    instrument_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn instrument_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Option<StoredInstrument>> {
    let stored = conn
        .query_row(
            &format!("SELECT instrument_json FROM compute_capacity_instruments {filter}"),
            parameters,
            |row| {
                let instrument_json: String = row.get(0)?;
                let instrument = from_json(&instrument_json, 0)?;
                Ok(StoredInstrument {
                    instrument,
                    instrument_json,
                })
            },
        )
        .optional()?;
    stored
        .map(|value| audit_instrument(conn, value))
        .transpose()
}

fn from_json<T: serde::de::DeserializeOwned>(json: &str, column: usize) -> rusqlite::Result<T> {
    serde_json::from_str(json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
    })
}

pub(super) fn canonical_json<T: serde::Serialize>(value: &T) -> Result<String> {
    crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256(
        value,
        256 * 1024,
    )
    .map(|(json, _)| json)
}

pub(super) fn activation_by_instrument_on(
    conn: &Connection,
    instrument_id: &str,
) -> Result<Option<StoredActivation>> {
    activation_on(conn, "WHERE instrument_id=?1", params![instrument_id])
}

pub(super) fn activation_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredActivation>> {
    activation_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn activation_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Option<StoredActivation>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT activation_receipt_json
                   FROM compute_capacity_instrument_activations {filter}"
            ),
            parameters,
            |row| {
                let activation_json: String = row.get(0)?;
                Ok(StoredActivation {
                    activation: from_json(&activation_json, 0)?,
                    activation_json,
                })
            },
        )
        .optional()?;
    stored
        .map(|value| audit_activation(conn, value))
        .transpose()
}

pub(super) fn retirement_by_instrument_on(
    conn: &Connection,
    instrument_id: &str,
) -> Result<Option<StoredRetirement>> {
    retirement_on(conn, "WHERE instrument_id=?1", params![instrument_id])
}

pub(super) fn retirement_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredRetirement>> {
    retirement_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn retirement_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Option<StoredRetirement>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT retirement_receipt_json
                   FROM compute_capacity_instrument_retirements {filter}"
            ),
            parameters,
            |row| {
                let retirement_json: String = row.get(0)?;
                Ok(StoredRetirement {
                    retirement: from_json(&retirement_json, 0)?,
                    retirement_json,
                })
            },
        )
        .optional()?;
    stored
        .map(|value| audit_retirement(conn, value))
        .transpose()
}

pub(super) fn adoption_by_offer_on(
    conn: &Connection,
    offer_id: &str,
) -> Result<Option<StoredAdoption>> {
    adoption_on(conn, "WHERE offer_id=?1", params![offer_id])
}

pub(super) fn adoption_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredAdoption>> {
    adoption_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn adoption_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Option<StoredAdoption>> {
    let stored = conn
        .query_row(
            &format!(
                "SELECT adoption_receipt_json
                   FROM compute_capacity_instrument_offer_adoptions {filter}"
            ),
            parameters,
            |row| {
                let adoption_json: String = row.get(0)?;
                Ok(StoredAdoption {
                    adoption: from_json(&adoption_json, 0)?,
                    adoption_json,
                })
            },
        )
        .optional()?;
    stored.map(|value| audit_adoption(conn, value)).transpose()
}

pub(super) fn currentness_on(
    conn: &Connection,
    instrument_id: &str,
) -> Result<Option<ComputeCapacityInstrumentCurrentnessReceipt>> {
    let Some(root) = instrument_by_id_on(conn, instrument_id)? else {
        return Ok(None);
    };
    let activation = activation_by_instrument_on(conn, instrument_id)?;
    let retirement = retirement_by_instrument_on(conn, instrument_id)?;
    let expected_status = if retirement.is_some() {
        COMPUTE_CAPACITY_INSTRUMENT_STATUS_RETIRED
    } else if activation.is_some() {
        COMPUTE_CAPACITY_INSTRUMENT_STATUS_ACTIVE
    } else {
        COMPUTE_CAPACITY_INSTRUMENT_STATUS_REGISTERED
    };
    let view_status: String = conn.query_row(
        "SELECT current_status FROM compute_capacity_instrument_current
          WHERE instrument_id=?1 AND instrument_revision=?2 AND instrument_digest=?3",
        params![
            instrument_id,
            root.instrument.instrument_revision,
            root.instrument.instrument_digest
        ],
        |row| row.get(0),
    )?;
    if view_status != expected_status {
        bail!("capacity instrument current view failed exact audit");
    }
    Ok(Some(ComputeCapacityInstrumentCurrentnessReceipt {
        schema: COMPUTE_CAPACITY_INSTRUMENT_CURRENTNESS_SCHEMA,
        instrument: root.instrument,
        current_status: expected_status.to_string(),
        activation: activation.map(|value| value.activation),
        retirement: retirement.map(|value| value.retirement),
    }))
}

impl Store {
    pub(crate) fn compute_capacity_instrument(
        &self,
        instrument_id: &str,
    ) -> Result<Option<ComputeCapacityInstrument>> {
        validate_exact(instrument_id, "capacity instrument ID", 200)?;
        let conn = self.conn()?;
        Ok(instrument_by_id_on(&conn, instrument_id)?.map(|value| value.instrument))
    }

    pub(crate) fn compute_capacity_instrument_currentness(
        &self,
        instrument_id: &str,
    ) -> Result<Option<ComputeCapacityInstrumentCurrentnessReceipt>> {
        validate_exact(instrument_id, "capacity instrument ID", 200)?;
        let conn = self.conn()?;
        currentness_on(&conn, instrument_id)
    }

    pub(crate) fn list_compute_capacity_instruments(
        &self,
        limit: usize,
    ) -> Result<Vec<ComputeCapacityInstrumentCurrentnessReceipt>> {
        let conn = self.conn()?;
        let mut statement = conn.prepare(
            "SELECT instrument_id FROM compute_capacity_instruments
              ORDER BY registered_at DESC, instrument_id ASC LIMIT ?1",
        )?;
        let ids = statement
            .query_map(params![limit.clamp(1, 100) as i64], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        ids.into_iter()
            .map(|id| currentness_on(&conn, &id)?.context("capacity instrument disappeared"))
            .collect()
    }

    pub(crate) fn compute_capacity_instrument_offer_adoption(
        &self,
        offer_id: &str,
    ) -> Result<Option<ComputeCapacityInstrumentOfferAdoptionReceipt>> {
        validate_exact(offer_id, "Offer ID", 200)?;
        let conn = self.conn()?;
        Ok(adoption_by_offer_on(&conn, offer_id)?.map(|value| value.adoption))
    }
}
