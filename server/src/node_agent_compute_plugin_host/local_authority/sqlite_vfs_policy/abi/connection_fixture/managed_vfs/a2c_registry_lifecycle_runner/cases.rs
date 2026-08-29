//! Fixed libtest selectors; no RegistryLifecycle identity is accepted from the environment.

macro_rules! exact_case {
    ($name:ident, $test:literal) => {
        pub(super) const $name: &str = concat!(
            "node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::abi::connection_fixture::managed_vfs::a2c_registry_lifecycle_runner::",
            $test
        );
    };
}

exact_case!(
    CALLBACK_COMPLETION_BEFORE,
    "registry_lifecycle_callback_completion_before"
);
exact_case!(
    CALLBACK_COMPLETION_NATIVE_UNCERTAIN,
    "registry_lifecycle_callback_completion_native_uncertain"
);
exact_case!(
    CALLBACK_COMPLETION_AFTER_SUCCESS_KNOWN,
    "registry_lifecycle_callback_completion_after_success_known"
);
exact_case!(
    CONNECTION_OBSERVATION_BEFORE,
    "registry_lifecycle_connection_observation_before"
);
exact_case!(
    CONNECTION_OBSERVATION_OUTSTANDING_SIDECAR,
    "registry_lifecycle_connection_observation_outstanding_sidecar"
);
exact_case!(
    CONNECTION_OBSERVATION_AFTER_SUCCESS_KNOWN,
    "registry_lifecycle_connection_observation_after_success_known"
);
exact_case!(
    REGISTRY_ROUTE_REMOVAL_BEFORE,
    "registry_lifecycle_registry_route_removal_before"
);
exact_case!(
    REGISTRY_ROUTE_REMOVAL_OWNER_NATIVE,
    "registry_lifecycle_registry_route_removal_owner_native"
);
exact_case!(
    REGISTRY_ROUTE_REMOVAL_PUBLISH_NATIVE,
    "registry_lifecycle_registry_route_removal_publish_native"
);
exact_case!(
    REGISTRY_ROUTE_REMOVAL_AFTER_SUCCESS_KNOWN,
    "registry_lifecycle_registry_route_removal_after_success_known"
);
exact_case!(
    LOGICAL_ROUTE_REMOVAL_BEFORE,
    "registry_lifecycle_logical_route_removal_before"
);
exact_case!(
    LOGICAL_ROUTE_REMOVAL_CLAIM_NATIVE,
    "registry_lifecycle_logical_route_removal_claim_native"
);
exact_case!(
    LOGICAL_ROUTE_REMOVAL_INDEX_NATIVE,
    "registry_lifecycle_logical_route_removal_index_native"
);
exact_case!(
    LOGICAL_ROUTE_REMOVAL_AFTER_SUCCESS_KNOWN,
    "registry_lifecycle_logical_route_removal_after_success_known"
);
exact_case!(
    SUCCESS_SHARED_NONFINAL,
    "registry_lifecycle_success_shared_nonfinal"
);
exact_case!(SUCCESS_FINAL, "registry_lifecycle_success_final");
