use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::TransactionBehavior;

use crate::node_compute_sharing::endpoint_authority::{
    AuthorizedNodeEndpointOwnerReauthentication, PreparedNodeEndpointOwnerReauthentication,
};

use super::{owner_reauthentication_receipt, NodeEndpointOwnerReauthenticationReceipt, Store};

pub(super) mod consumption_rows;
mod currentness;
mod rows;

pub(super) fn record(
    store: &Store,
    authorized: &AuthorizedNodeEndpointOwnerReauthentication,
) -> Result<NodeEndpointOwnerReauthenticationReceipt> {
    let mut connection = store.conn()?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let receipt = record_at_on(&transaction, authorized, Utc::now())?;
    transaction.commit()?;
    Ok(receipt)
}

pub(super) fn record_at_on(
    transaction: &rusqlite::Transaction<'_>,
    authorized: &AuthorizedNodeEndpointOwnerReauthentication,
    recorded_at: DateTime<Utc>,
) -> Result<NodeEndpointOwnerReauthenticationReceipt> {
    if let Some(stored) = rows::by_issuance_request_on(
        transaction,
        authorized.owner_user_id(),
        authorized.authorization_issuance_request_id(),
    )? {
        let recorded_at = parse_recorded_at(stored.envelope().recorded_at())?;
        let prepared = authorized.prepare(recorded_at)?;
        stored.ensure_exact(&prepared)?;
        let receipt = owner_reauthentication_receipt(
            stored.into_envelope(),
            prepared.receipt_digest().to_string(),
            true,
        );
        return Ok(receipt);
    }

    let prepared = authorized.prepare(recorded_at)?;
    currentness::require_current_sources_on(transaction, &prepared, recorded_at)?;
    rows::insert_on(transaction, &prepared)?;
    let stored = rows::by_receipt_id_on(
        transaction,
        prepared.envelope().reauthentication_receipt_id(),
    )?
    .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_READBACK_MISSING"))?;
    stored.ensure_exact(&prepared)?;
    currentness::require_current_sources_on(transaction, &prepared, recorded_at)?;
    let receipt = owner_reauthentication_receipt(
        stored.into_envelope(),
        prepared.receipt_digest().to_string(),
        false,
    );
    Ok(receipt)
}

fn parse_recorded_at(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(Into::into)
}

pub(super) fn receipt_by_id_on(
    connection: &rusqlite::Connection,
    receipt_id: &str,
) -> Result<
    Option<(
        crate::node_compute_sharing::endpoint_authority::NodeEndpointOwnerReauthenticationEnvelope,
        String,
    )>,
> {
    rows::by_receipt_id_on(connection, receipt_id)?
        .map(|stored| {
            let digest = stored.receipt_digest().to_string();
            Ok((stored.into_envelope(), digest))
        })
        .transpose()
}
