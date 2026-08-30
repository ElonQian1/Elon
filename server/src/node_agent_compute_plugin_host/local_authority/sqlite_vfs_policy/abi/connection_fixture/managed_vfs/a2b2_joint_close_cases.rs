//! Fixed libtest selectors for the frozen JointClose family.

use super::a2b2_cases::JointCloseSelector;

#[derive(Clone, Copy)]
pub(super) struct ExactJointCloseCase {
    pub(super) selector: JointCloseSelector,
    pub(super) exact_test: &'static str,
}

macro_rules! exact_cases {
    ($(($name:ident, $selector:ident, $test:ident)),+ $(,)?) => {
        $(
            pub(super) const $name: &str = concat!(
                "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2b2_joint_close_runner::",
                stringify!($test),
            );
        )+

        pub(super) const ALL: [ExactJointCloseCase; JointCloseSelector::ALL.len()] = [
            $(ExactJointCloseCase {
                selector: JointCloseSelector::$selector,
                exact_test: $name,
            }),+
        ];
    };
}

exact_cases!(
    (
        RAW_STATE_TAKE_REJECTED,
        RawStateTakeRejected,
        raw_state_take_rejected
    ),
    (
        BEGIN_CONNECTION_CLOSE_REJECTED,
        BeginConnectionCloseRejected,
        begin_connection_close_rejected
    ),
    (
        CALLBACK_ADMISSION_REJECTED,
        CallbackAdmissionRejected,
        callback_admission_rejected
    ),
    (
        CALLBACK_WRAPPER_BEFORE,
        CallbackWrapperBefore,
        callback_wrapper_before
    ),
    (
        SHM_VIEW_UNMAP_BEFORE,
        ShmViewUnmapBefore,
        shm_view_unmap_before
    ),
    (
        SHM_VIEW_UNMAP_NATIVE_UNCERTAIN,
        ShmViewUnmapNativeUncertain,
        shm_view_unmap_native_uncertain
    ),
    (
        SHM_VIEW_UNMAP_AFTER_KNOWN,
        ShmViewUnmapAfterKnown,
        shm_view_unmap_after_known
    ),
    (
        SHM_VIEW_UNMAP_AFTER_UNCERTAIN,
        ShmViewUnmapAfterUncertain,
        shm_view_unmap_after_uncertain
    ),
    (
        SHM_MAPPING_CLOSE_BEFORE,
        ShmMappingCloseBefore,
        shm_mapping_close_before
    ),
    (
        SHM_MAPPING_CLOSE_NATIVE_UNCERTAIN,
        ShmMappingCloseNativeUncertain,
        shm_mapping_close_native_uncertain
    ),
    (
        SHM_MAPPING_CLOSE_AFTER_KNOWN,
        ShmMappingCloseAfterKnown,
        shm_mapping_close_after_known
    ),
    (
        SHM_MAPPING_CLOSE_AFTER_UNCERTAIN,
        ShmMappingCloseAfterUncertain,
        shm_mapping_close_after_uncertain
    ),
    (
        SHM_DMS_RELEASE_BEFORE,
        ShmDmsReleaseBefore,
        shm_dms_release_before
    ),
    (
        SHM_DMS_RELEASE_NATIVE_UNCERTAIN,
        ShmDmsReleaseNativeUncertain,
        shm_dms_release_native_uncertain
    ),
    (
        SHM_DMS_RELEASE_AFTER_KNOWN,
        ShmDmsReleaseAfterKnown,
        shm_dms_release_after_known
    ),
    (
        SHM_DMS_RELEASE_AFTER_UNCERTAIN,
        ShmDmsReleaseAfterUncertain,
        shm_dms_release_after_uncertain
    ),
    (
        SHM_FILE_CLOSE_BEFORE,
        ShmFileCloseBefore,
        shm_file_close_before
    ),
    (
        SHM_FILE_CLOSE_NATIVE_RETRYABLE,
        ShmFileCloseNativeRetryable,
        shm_file_close_native_retryable
    ),
    (
        SHM_FILE_CLOSE_NATIVE_UNCERTAIN,
        ShmFileCloseNativeUncertain,
        shm_file_close_native_uncertain
    ),
    (
        SHM_FILE_CLOSE_AFTER_KNOWN,
        ShmFileCloseAfterKnown,
        shm_file_close_after_known
    ),
    (
        SHM_FILE_CLOSE_AFTER_UNCERTAIN,
        ShmFileCloseAfterUncertain,
        shm_file_close_after_uncertain
    ),
    (SHM_DETACH_BEFORE, ShmDetachBefore, shm_detach_before),
    (
        SHM_DETACH_AFTER_KNOWN,
        ShmDetachAfterKnown,
        shm_detach_after_known
    ),
    (
        SHM_DETACH_AFTER_UNCERTAIN,
        ShmDetachAfterUncertain,
        shm_detach_after_uncertain
    ),
    (
        MAIN_LOCK_RELEASE_BEFORE,
        MainLockReleaseBefore,
        main_lock_release_before
    ),
    (
        MAIN_LOCK_RELEASE_NATIVE_UNCERTAIN_SHARED,
        MainLockReleaseNativeUncertainShared,
        main_lock_release_native_uncertain_shared
    ),
    (
        MAIN_LOCK_RELEASE_NATIVE_UNCERTAIN_RESERVED,
        MainLockReleaseNativeUncertainReserved,
        main_lock_release_native_uncertain_reserved
    ),
    (
        MAIN_LOCK_RELEASE_AFTER_KNOWN,
        MainLockReleaseAfterKnown,
        main_lock_release_after_known
    ),
    (
        MAIN_FILE_CLOSE_BEFORE,
        MainFileCloseBefore,
        main_file_close_before
    ),
    (
        MAIN_FILE_CLOSE_NATIVE_RETRYABLE,
        MainFileCloseNativeRetryable,
        main_file_close_native_retryable
    ),
    (
        MAIN_FILE_CLOSE_NATIVE_UNCERTAIN,
        MainFileCloseNativeUncertain,
        main_file_close_native_uncertain
    ),
    (
        MAIN_FILE_CLOSE_AFTER_KNOWN,
        MainFileCloseAfterKnown,
        main_file_close_after_known
    ),
    (PHYSICAL_SUCCESS, PhysicalSuccess, physical_success),
    (
        REGISTRY_WAL_MAIN_CLOSE_BEFORE,
        RegistryWalMainCloseBefore,
        registry_wal_main_close_before
    ),
    (
        REGISTRY_WAL_MAIN_CLOSE_NATIVE_UNCERTAIN,
        RegistryWalMainCloseNativeUncertain,
        registry_wal_main_close_native_uncertain
    ),
    (
        REGISTRY_WAL_MAIN_CLOSE_AFTER_KNOWN,
        RegistryWalMainCloseAfterKnown,
        registry_wal_main_close_after_known
    ),
);

pub(super) fn validate_all() -> Result<(), &'static str> {
    for (index, case) in ALL.iter().enumerate() {
        if case.selector != JointCloseSelector::ALL[index] {
            return Err("A2_JOINT_CLOSE_FAMILY_CASE_ORDER_MISMATCH");
        }
        if ALL[..index]
            .iter()
            .any(|prior| prior.exact_test == case.exact_test)
        {
            return Err("A2_JOINT_CLOSE_FAMILY_EXACT_TEST_ALIAS");
        }
    }
    Ok(())
}
