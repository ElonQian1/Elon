//! Dormant, activation-rooted Provider active-successor contracts.
//!
//! This Domain does not activate a Provider or mint route, executor, dispatch, or market
//! authority. Durable creation remains unreachable until V277 supplies its atomic activation
//! witness and process-custody producer.

mod canonical;
mod policy;
mod types;
mod validation;

pub(crate) use canonical::*;
pub(crate) use policy::*;
pub(crate) use types::*;
pub(crate) use validation::*;
