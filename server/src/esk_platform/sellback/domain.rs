//! Pure domain facade shared by production and the SQLite harness, without HTTP dependencies.
#[path = "model.rs"]
mod model;
#[path = "policy.rs"]
mod policy;
#[path = "validation.rs"]
mod validation;

pub(crate) use model::*;
pub(crate) use policy::*;
pub(crate) use validation::*;

#[cfg(test)]
#[path = "domain_tests.rs"]
pub(crate) mod tests;
