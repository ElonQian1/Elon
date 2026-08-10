//! Static A2b1 evidence records. These tests describe required outcomes; they are not dynamic
//! SQLite or Win32 execution evidence.

mod lock;
mod map;
mod model;
mod operation;

#[test]
fn a2b1_declared_static_subset_is_self_consistent() {
    model::validate_matrix(map::CASES, lock::CASES).expect("valid A2b1 static case matrix");
}
