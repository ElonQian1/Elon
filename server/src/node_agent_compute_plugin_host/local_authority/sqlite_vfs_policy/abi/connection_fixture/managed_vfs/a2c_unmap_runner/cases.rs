//! Fixed libtest selectors for the implemented SharedNonFinal Unmap slice.

macro_rules! exact_case {
    ($name:ident, $test:literal) => {
        pub(super) const $name: &str = concat!(
            "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2c_unmap_runner::",
            $test
        );
    };
}

exact_case!(
    SHARED_DELETE_REQUEST_VALIDATION,
    "unmap_shared_delete_request_validation"
);
exact_case!(
    SHARED_KEEP_CALLBACK_ADMISSION,
    "unmap_shared_keep_callback_admission"
);
exact_case!(
    SHARED_KEEP_CALLBACK_WRAPPER_BEFORE,
    "unmap_shared_keep_callback_wrapper_before"
);
exact_case!(
    SHARED_KEEP_HELD_SHARED_LOCK,
    "unmap_shared_keep_held_shared_lock"
);
exact_case!(
    SHARED_KEEP_HELD_EXCLUSIVE_LOCK,
    "unmap_shared_keep_held_exclusive_lock"
);
exact_case!(SHARED_KEEP_DETACH_BEFORE, "unmap_shared_keep_detach_before");
exact_case!(
    SHARED_KEEP_DETACH_AFTER_KNOWN,
    "unmap_shared_keep_detach_after_known"
);
exact_case!(
    SHARED_KEEP_DETACH_AFTER_UNCERTAIN,
    "unmap_shared_keep_detach_after_uncertain"
);
exact_case!(
    SHARED_KEEP_COMPLETION_NATIVE_UNCERTAIN,
    "unmap_shared_keep_completion_native_uncertain"
);
exact_case!(SHARED_KEEP_SUCCESS, "unmap_shared_keep_success");
exact_case!(SHARED_DELETE_SUCCESS, "unmap_shared_delete_success");
