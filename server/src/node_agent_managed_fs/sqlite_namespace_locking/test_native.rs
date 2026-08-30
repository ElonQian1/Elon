use super::*;
use crate::node_agent_managed_fs::sqlite_namespace::{
    lock_domain::ManagedSqliteHeldLocks, ManagedSqliteMainCloseTestNativeEvidence,
    ManagedSqliteMainCloseTestNativeObservation, ManagedSqliteMainLockHeldRangePrestate,
    ManagedSqliteMainLockOffsetClass,
};

pub(in crate::node_agent_managed_fs::sqlite_namespace) struct ManagedSqliteMainUnlockTestNativeResult
{
    pub(in crate::node_agent_managed_fs::sqlite_namespace) result:
        Result<(), ManagedSqliteLockFailure>,
    pub(in crate::node_agent_managed_fs::sqlite_namespace) evidence:
        Option<ManagedSqliteMainCloseTestNativeEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SelectedMainCloseUnlock {
    prestate: ManagedSqliteMainLockHeldRangePrestate,
    offset_class: ManagedSqliteMainLockOffsetClass,
    offset: u64,
    length: u64,
    failure_phase: ManagedSqliteLockFailurePhase,
}

impl PinnedManagedSqliteMainFile {
    pub(in crate::node_agent_managed_fs::sqlite_namespace) fn unlock_to_for_main_close_test_native(
        &mut self,
        offset_class: ManagedSqliteMainLockOffsetClass,
    ) -> ManagedSqliteMainUnlockTestNativeResult {
        let file = &self.file.file;
        let mut domain = match self.live_lock_domain() {
            Ok(domain) => domain,
            Err(failure) => {
                return ManagedSqliteMainUnlockTestNativeResult {
                    result: Err(failure),
                    evidence: None,
                };
            }
        };
        let Some(selected) = select_main_close_unlock(domain.held(), offset_class) else {
            return ManagedSqliteMainUnlockTestNativeResult {
                result: Err(ManagedSqliteLockFailure::message(
                    ManagedSqliteLockFailurePhase::RequestValidation,
                    ManagedSqliteLockFailureKind::InvalidTransition,
                    "NODE_MANAGED_SQLITE_MAIN_CLOSE_NATIVE_LOCK_PRESTATE_INVALID",
                    false,
                )),
                evidence: None,
            };
        };
        let unavailable =
            platform::unlock_sqlite_byte_range_return_receipt_unavailable_for_main_close_test(
                file,
                selected.offset,
                selected.length,
            );
        debug_assert_eq!(unavailable.offset, selected.offset);
        debug_assert_eq!(unavailable.length, selected.length);
        let evidence = ManagedSqliteMainCloseTestNativeEvidence::MainLockRelease {
            held_range_prestate: selected.prestate,
            selected_offset_class: selected.offset_class,
            exact_call_occurrence: unavailable.exact_call_occurrence,
            observation: ManagedSqliteMainCloseTestNativeObservation::ReturnReceiptUnavailable,
        };
        ManagedSqliteMainUnlockTestNativeResult {
            result: Err(poison(
                &mut domain,
                selected.failure_phase,
                unavailable.error,
            )),
            evidence: Some(evidence),
        }
    }
}

fn select_main_close_unlock(
    held: &ManagedSqliteHeldLocks,
    offset_class: ManagedSqliteMainLockOffsetClass,
) -> Option<SelectedMainCloseUnlock> {
    if held.terminal || held.pending || held.exclusive || !held.shared {
        return None;
    }
    match (offset_class, held.reserved) {
        (ManagedSqliteMainLockOffsetClass::SharedRange, false) => Some(SelectedMainCloseUnlock {
            prestate: ManagedSqliteMainLockHeldRangePrestate::Shared,
            offset_class,
            offset: SHARED_FIRST,
            length: SHARED_SIZE,
            failure_phase: ManagedSqliteLockFailurePhase::ReleaseShared,
        }),
        (ManagedSqliteMainLockOffsetClass::ReservedByte, true) => Some(SelectedMainCloseUnlock {
            prestate: ManagedSqliteMainLockHeldRangePrestate::ReservedShared,
            offset_class,
            offset: RESERVED_BYTE,
            length: 1,
            failure_phase: ManagedSqliteLockFailurePhase::ReleaseReserved,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_first_production_unlock_for_exact_shared_or_reserved_prestate_is_selectable() {
        let mut held = ManagedSqliteHeldLocks {
            shared: true,
            ..ManagedSqliteHeldLocks::default()
        };
        let shared = select_main_close_unlock(&held, ManagedSqliteMainLockOffsetClass::SharedRange)
            .expect("shared-only prestate selects the production shared-range release");
        assert_eq!(
            shared.prestate,
            ManagedSqliteMainLockHeldRangePrestate::Shared
        );
        assert_eq!((shared.offset, shared.length), (SHARED_FIRST, SHARED_SIZE));
        assert!(
            select_main_close_unlock(&held, ManagedSqliteMainLockOffsetClass::ReservedByte)
                .is_none()
        );

        held.reserved = true;
        let reserved =
            select_main_close_unlock(&held, ManagedSqliteMainLockOffsetClass::ReservedByte).expect(
                "reserved+shared prestate selects the first production reserved-byte release",
            );
        assert_eq!(
            reserved.prestate,
            ManagedSqliteMainLockHeldRangePrestate::ReservedShared
        );
        assert_eq!((reserved.offset, reserved.length), (RESERVED_BYTE, 1));
        assert!(
            select_main_close_unlock(&held, ManagedSqliteMainLockOffsetClass::SharedRange)
                .is_none()
        );

        held.pending = true;
        assert!(
            select_main_close_unlock(&held, ManagedSqliteMainLockOffsetClass::ReservedByte)
                .is_none()
        );
    }
}
