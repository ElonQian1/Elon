//! Transactional authority for immutable capacity instruments and exact Offer adoption.

mod adoption;
mod audit;
mod canonical;
mod historical;
mod read;
mod types;
mod validation;
mod write;

pub(in crate::store) use adoption::{
    require_capacity_instrument_adoption_for_historical_offer_on,
    require_current_capacity_instrument_adoption_on,
};
pub(in crate::store) use historical::{
    audited_historical_capacity_instrument_settlement_source_on,
    HistoricalCapacityInstrumentSettlementSource,
};
pub(crate) use types::*;
