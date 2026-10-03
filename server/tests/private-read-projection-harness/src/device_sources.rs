//! Exercise the production model/storage, not a copied implementation.
#[path = "../../../src/grid_device_sources/model.rs"]
mod model;
#[path = "../../../src/grid_device_sources/storage.rs"]
mod storage;
#[cfg(test)]
#[path = "../../../src/grid_device_sources/tests.rs"]
mod tests;
