//! Sixteen process-isolated RegistryLifecycle Windows dynamic runners.

use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{anyhow, Context};

use super::{
    a2_dynamic_evidence::{
        ChildLaunchIdentity, DynamicChildFailure, ValidatedParentCleanupReceipt,
        ValidatedWindowsDynamicRecord, WindowsDynamicEnvironment, A2_DYNAMIC_CHILD_NONCE_ENV,
    },
    a2b2_cases::{validate_registry_lifecycle_report_payload, RegistryLifecycleSelector},
};

mod cases;
mod child;

macro_rules! registry_lifecycle_case {
    ($test:ident, $exact:ident, $selector:ident) => {
        #[test]
        fn $test() -> anyhow::Result<()> {
            run_isolated_case(cases::$exact, RegistryLifecycleSelector::$selector)
        }
    };
}

registry_lifecycle_case!(
    registry_lifecycle_callback_completion_before,
    CALLBACK_COMPLETION_BEFORE,
    CallbackCompletionBefore
);
registry_lifecycle_case!(
    registry_lifecycle_callback_completion_native_uncertain,
    CALLBACK_COMPLETION_NATIVE_UNCERTAIN,
    CallbackCompletionNativeUncertain
);
registry_lifecycle_case!(
    registry_lifecycle_callback_completion_after_success_known,
    CALLBACK_COMPLETION_AFTER_SUCCESS_KNOWN,
    CallbackCompletionAfterSuccessKnown
);
registry_lifecycle_case!(
    registry_lifecycle_connection_observation_before,
    CONNECTION_OBSERVATION_BEFORE,
    ConnectionObservationBefore
);
registry_lifecycle_case!(
    registry_lifecycle_connection_observation_outstanding_sidecar,
    CONNECTION_OBSERVATION_OUTSTANDING_SIDECAR,
    ConnectionObservationOutstandingSidecar
);
registry_lifecycle_case!(
    registry_lifecycle_connection_observation_after_success_known,
    CONNECTION_OBSERVATION_AFTER_SUCCESS_KNOWN,
    ConnectionObservationAfterSuccessKnown
);
registry_lifecycle_case!(
    registry_lifecycle_registry_route_removal_before,
    REGISTRY_ROUTE_REMOVAL_BEFORE,
    RegistryRouteRemovalBefore
);
registry_lifecycle_case!(
    registry_lifecycle_registry_route_removal_owner_native,
    REGISTRY_ROUTE_REMOVAL_OWNER_NATIVE,
    RegistryRouteRemovalOwnerNative
);
registry_lifecycle_case!(
    registry_lifecycle_registry_route_removal_publish_native,
    REGISTRY_ROUTE_REMOVAL_PUBLISH_NATIVE,
    RegistryRouteRemovalPublishNative
);
registry_lifecycle_case!(
    registry_lifecycle_registry_route_removal_after_success_known,
    REGISTRY_ROUTE_REMOVAL_AFTER_SUCCESS_KNOWN,
    RegistryRouteRemovalAfterSuccessKnown
);
registry_lifecycle_case!(
    registry_lifecycle_logical_route_removal_before,
    LOGICAL_ROUTE_REMOVAL_BEFORE,
    LogicalRouteRemovalBefore
);
registry_lifecycle_case!(
    registry_lifecycle_logical_route_removal_claim_native,
    LOGICAL_ROUTE_REMOVAL_CLAIM_NATIVE,
    LogicalRouteRemovalClaimNative
);
registry_lifecycle_case!(
    registry_lifecycle_logical_route_removal_index_native,
    LOGICAL_ROUTE_REMOVAL_INDEX_NATIVE,
    LogicalRouteRemovalIndexNative
);
registry_lifecycle_case!(
    registry_lifecycle_logical_route_removal_after_success_known,
    LOGICAL_ROUTE_REMOVAL_AFTER_SUCCESS_KNOWN,
    LogicalRouteRemovalAfterSuccessKnown
);
registry_lifecycle_case!(
    registry_lifecycle_success_shared_nonfinal,
    SUCCESS_SHARED_NONFINAL,
    SuccessSharedNonFinal
);
registry_lifecycle_case!(
    registry_lifecycle_success_final,
    SUCCESS_FINAL,
    SuccessFinal
);

fn run_isolated_case(
    exact_test: &'static str,
    selector: RegistryLifecycleSelector,
) -> anyhow::Result<()> {
    if let Some(root) = child::selected_child_root()? {
        return child::exercise_child(&root, selector);
    }
    run_parent(exact_test, selector)
}

fn run_parent(exact_test: &'static str, selector: RegistryLifecycleSelector) -> anyhow::Result<()> {
    let executable =
        std::env::current_exe().context("resolve current RegistryLifecycle test executable")?;
    let root = create_private_child_root(selector)?;
    let launch = ChildLaunchIdentity::new();
    let spawned = match Command::new(executable)
        .args(["--exact", exact_test, "--nocapture"])
        .env(child::CHILD_ROOT_ENV, &root)
        .env(A2_DYNAMIC_CHILD_NONCE_ENV, launch.env_value())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(spawned) => spawned,
        Err(error) => {
            let error = anyhow!(error).context("spawn isolated RegistryLifecycle child");
            return Err(cleanup_failed_case(&root, error));
        }
    };

    let bound = match launch.bind(spawned) {
        Ok(bound) => bound,
        Err(error) => return Err(handle_child_failure(&root, error)),
    };
    let child = match bound.wait_for_successful_report() {
        Ok(child) => child,
        Err(error) => return Err(handle_child_failure(&root, error)),
    };
    let result = (|| {
        let observation =
            validate_registry_lifecycle_report_payload(selector, child.actual_payload())
                .map_err(anyhow::Error::msg)?;
        let environment =
            WindowsDynamicEnvironment::capture(&root, &child).map_err(anyhow::Error::msg)?;
        let cleanup = ValidatedParentCleanupReceipt::remove_after_child_exit(&child, &environment)
            .map_err(anyhow::Error::msg)?;
        let record = ValidatedWindowsDynamicRecord::validate_registry_lifecycle(
            observation,
            environment,
            child,
            cleanup,
        )
        .map_err(anyhow::Error::msg)?;
        let report = record.report();
        if report.case_selector() != selector.report_name() || !report.parent_cleanup_deleted() {
            return Err(anyhow!("RegistryLifecycle evidence report binding changed"));
        }
        println!("{report}");
        Ok(())
    })();
    match result {
        Ok(()) => Ok(()),
        Err(error) => Err(cleanup_failed_case(&root, error)),
    }
}

fn create_private_child_root(
    selector: RegistryLifecycleSelector,
) -> anyhow::Result<std::path::PathBuf> {
    let requested = std::env::temp_dir().join(format!(
        "elon-a2-registry-lifecycle-{}-{}-{}",
        selector.report_name(),
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir(&requested).with_context(|| {
        format!(
            "create fresh parent-owned RegistryLifecycle root at {}",
            requested.display()
        )
    })?;
    match fs::canonicalize(&requested) {
        Ok(root) if root.is_absolute() => Ok(root),
        Ok(_) => {
            let error = anyhow!("canonical RegistryLifecycle root is not absolute");
            Err(cleanup_failed_case(&requested, error))
        }
        Err(error) => {
            let error =
                anyhow!(error).context("canonicalize fresh parent-owned RegistryLifecycle root");
            Err(cleanup_failed_case(&requested, error))
        }
    }
}

fn handle_child_failure(root: &Path, failure: DynamicChildFailure) -> anyhow::Error {
    let exit_confirmed = failure.exit_confirmed();
    let error = anyhow!(failure);
    if exit_confirmed {
        cleanup_failed_case(root, error)
    } else {
        error.context(format!(
            "retained RegistryLifecycle case root {} because child exit is unconfirmed",
            root.display()
        ))
    }
}

fn cleanup_failed_case(root: &Path, error: anyhow::Error) -> anyhow::Error {
    match fs::remove_dir_all(root) {
        Ok(()) => error,
        Err(cleanup_error) if cleanup_error.kind() == std::io::ErrorKind::NotFound => error,
        Err(cleanup_error) => error.context(format!(
            "also failed to remove RegistryLifecycle case root {}: {cleanup_error}",
            root.display()
        )),
    }
}
