//! Compile the real authorization domain without the HTTP service assembly.
#[path = "../../../src/esk_platform/access/migration.rs"]
pub(crate) mod migration;
#[path = "../../../src/esk_platform/access/model.rs"]
mod model;
#[path = "../../../src/esk_platform/access/validation.rs"]
mod validation;

pub(crate) use model::*;
pub(crate) use validation::*;

#[cfg(test)]
#[path = "../../../src/esk_platform/access/tests_validation.rs"]
mod tests_validation;
