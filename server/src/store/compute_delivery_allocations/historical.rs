use anyhow::{anyhow, Result};
use rusqlite::Connection;

use crate::compute_federation::delivery_allocation::{
    ComputeDeliveryAllocationGrant, ComputeDeliveryAllocationTerminalReceipt,
};

use super::{
    read::{
        audit_historical_exercise_consumers_on,
        persisted_historical_delivery_allocation_reservation_authority_on,
        raw_terminal_by_grant_on,
    },
    types::DeliveryAllocationReservationAuthority,
};

/// Audited v228 owners plus the sealed whole-transfer authority reconstructed from historical
/// Claim and ledger rows. This type never crosses the Store boundary.
pub(in crate::store) struct HistoricalDeliveryAllocationSettlementSource {
    grant: ComputeDeliveryAllocationGrant,
    terminal: ComputeDeliveryAllocationTerminalReceipt,
    authority: DeliveryAllocationReservationAuthority,
}

impl HistoricalDeliveryAllocationSettlementSource {
    pub(in crate::store) fn grant(&self) -> &ComputeDeliveryAllocationGrant {
        &self.grant
    }

    pub(in crate::store) fn terminal(&self) -> &ComputeDeliveryAllocationTerminalReceipt {
        &self.terminal
    }

    pub(in crate::store) fn authority(&self) -> &DeliveryAllocationReservationAuthority {
        &self.authority
    }
}

pub(in crate::store) fn audited_historical_delivery_allocation_settlement_source_on(
    conn: &Connection,
    reservation_id: &str,
    reservation_claim_id: &str,
) -> Result<Option<HistoricalDeliveryAllocationSettlementSource>> {
    let Some(authority) = persisted_historical_delivery_allocation_reservation_authority_on(
        conn,
        reservation_id,
        reservation_claim_id,
    )?
    else {
        return Ok(None);
    };
    let grant = authority.transfer().grant().clone();
    let terminal = raw_terminal_by_grant_on(conn, &grant)?
        .ok_or_else(|| anyhow!("historical DeliveryAllocation authority lacks terminal owner"))?;
    audit_historical_exercise_consumers_on(conn, &grant, &terminal)?;
    Ok(Some(HistoricalDeliveryAllocationSettlementSource {
        grant,
        terminal,
        authority,
    }))
}
