//! Chain-off shadow settlement for accepted AI work and metered commerce usage.

pub(crate) mod api;
pub(crate) mod ledger;
pub(crate) mod model;
mod service;
mod sui_projection;

pub(crate) use service::{
    capture_commerce_invocation, capture_task_assignment, post_accepted_matter,
    void_canceled_matter,
};
