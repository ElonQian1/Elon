//! Internal contracts for the interactive desktop execution plane.
//!
//! These types deliberately do not add a batch workload kind, public route, persistence,
//! signaling transport, media transport, input injector, billing write, or production switch.

mod authority;
mod authority_head;
pub(crate) mod authority_record;
pub(crate) mod canonical;
pub(crate) mod metering;
pub(crate) mod offer;
pub(crate) mod product_authority;
pub(crate) mod reservation;
pub(crate) mod session;

pub(crate) const INTERACTIVE_DESKTOP_SERVICE_CLASS: &str = "interactive_desktop";

#[cfg(test)]
mod authority_head_tests;
#[cfg(test)]
mod authority_tests;
#[cfg(test)]
mod market_authority_tests;
#[cfg(test)]
mod metering_tests;
#[cfg(test)]
mod reservation_test_support;
#[cfg(test)]
mod reservation_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
