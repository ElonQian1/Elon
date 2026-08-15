mod child;
mod host;
mod receipt;
mod request;
mod wire;

pub use child::{
    ExternalPoolAdapterTaskProtocolChild, ExternalPoolAdapterTaskProtocolChildExchange,
};
pub use host::{ExternalPoolAdapterTaskProtocolHost, ExternalPoolAdapterTaskProtocolHostExchange};
pub use receipt::ExternalPoolAdapterTaskProtocolHostReceipt;
pub use request::{
    prepare_external_pool_adapter_task_request, PreparedExternalPoolAdapterTaskRequest,
};

use anyhow::{bail, Result};

/// The only ELTP v1 operations. Authenticated ACK is the aggregate of exchange receipts.
#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum ExternalPoolAdapterTaskOperationKind {
    Prepare = 1,
    IdempotentCommit = 2,
    CancelNoStart = 3,
    Reconcile = 4,
    AuthenticatedEvents = 5,
}

impl ExternalPoolAdapterTaskOperationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Prepare => "prepare",
            Self::IdempotentCommit => "idempotent_commit",
            Self::CancelNoStart => "cancel_no_start",
            Self::Reconcile => "reconcile",
            Self::AuthenticatedEvents => "authenticated_events",
        }
    }

    pub(super) fn from_wire(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Self::Prepare),
            2 => Ok(Self::IdempotentCommit),
            3 => Ok(Self::CancelNoStart),
            4 => Ok(Self::Reconcile),
            5 => Ok(Self::AuthenticatedEvents),
            _ => bail!("ELTP operation rejected"),
        }
    }
}
