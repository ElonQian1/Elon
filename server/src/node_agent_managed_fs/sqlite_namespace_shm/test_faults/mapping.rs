use std::ptr::NonNull;

use super::super::{
    coordinator::{ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState},
    mapping::ManagedSqliteShmRegionMapping,
    platform_shm,
    types::{ManagedSqliteShmFailure, ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase},
};

impl ManagedSqliteShmCoordinator {
    pub(in crate::node_agent_managed_fs::sqlite_namespace::shm) fn retain_test_mapping_custody(
        &self,
        state: &mut ManagedSqliteShmCoordinatorState,
        mapping: platform_shm::OwnedSqliteShmMapping,
        view: Option<platform_shm::OwnedSqliteShmView>,
        logical_pointer: Option<NonNull<u8>>,
        logical_length: usize,
        mapped_length: usize,
        aligned_offset: u64,
    ) -> Result<(), ManagedSqliteShmFailure> {
        let node = state.node.as_mut().ok_or_else(|| {
            ManagedSqliteShmFailure::code(
                ManagedSqliteShmFailurePhase::Gate,
                ManagedSqliteShmFailureClass::ProtocolViolation,
                "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_AT_TEST_MAPPING_FAULT",
            )
        })?;
        node.regions.push(ManagedSqliteShmRegionMapping {
            mapping,
            view,
            logical_pointer,
            logical_length,
            mapped_length,
            aligned_offset,
        });
        Ok(())
    }
}
