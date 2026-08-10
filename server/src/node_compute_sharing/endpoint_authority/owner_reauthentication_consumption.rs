//! Single-use closure between one exact owner reauthentication and one credential mutation.
//!
//! This module only seals canonical inputs. No Store, HTTP, transport, or credential mutation
//! caller is wired in this batch.

mod accessors;
mod contracts;
mod envelope_validation;
mod prepare;
mod validation;

pub(crate) use contracts::{
    NodeEndpointCredentialMutationResultBinding,
    NodeEndpointOwnerReauthenticationConsumptionEnvelope,
    PreparedNodeEndpointOwnerReauthenticationConsumption,
};
pub(crate) use prepare::prepare_owner_reauthentication_consumption;
