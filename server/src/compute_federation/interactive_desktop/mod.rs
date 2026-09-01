//! Internal contracts for the interactive desktop execution plane.
//!
//! These types deliberately do not add a batch workload kind, public route, persistence,
//! signaling transport, media transport, input injector, billing write, or production switch.

mod authority;
pub(crate) mod metering;
pub(crate) mod offer;
pub(crate) mod session;

pub(crate) const INTERACTIVE_DESKTOP_SERVICE_CLASS: &str = "interactive_desktop";

#[cfg(test)]
mod authority_tests;
#[cfg(test)]
mod metering_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
