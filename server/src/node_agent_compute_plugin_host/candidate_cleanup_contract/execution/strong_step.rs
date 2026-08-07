use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::{bail, Error, Result};

use super::{delete_steps::PendingCleanupObject, CandidateCleanupExecutionState};
use crate::node_agent_compute_plugin_host::candidate_cleanup_contract::{
    journal::build_initial_delete_intent, validate_hashed_execution_plan,
    DurableCandidateCleanupDeleteIntent, HashedComputePluginCandidateCleanupExecutionPlan,
    HashedComputePluginCandidateCleanupStepEvent,
};
use crate::node_agent_managed_fs::{ManagedDeleteDisposition, ManagedObjectBinding};

/// One exact object accepted delete disposition while the original root lock and every remaining
/// object handle stay retained. This does not prove parent-relative absence or namespace durability.
#[must_use = "physical disposition must be journaled or retained for recovery"]
pub(in crate::node_agent_compute_plugin_host) struct PhysicallyDisposedCandidateCleanupObject {
    state: CandidateCleanupExecutionState,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
    intent_event: HashedComputePluginCandidateCleanupStepEvent,
    disposition: ManagedDeleteDisposition,
    disposition_set_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupDispositionFailurePhase {
    RejectedBeforeDisposition,
    DispositionFailed,
}

pub(in crate::node_agent_compute_plugin_host) enum CandidateCleanupDispositionFailureCustody {
    Intent(DurableCandidateCleanupDeleteIntent),
    Rejected(CandidateCleanupDispositionRejectedCustody),
    Retry(CandidateCleanupDispositionRetryCustody),
}

/// An impossible post-validation shape is retained without a constructor that could recreate a
/// sealed topology or durable intent. Operator recovery must fail closed on this value.
#[must_use = "rejected disposition custody must be retained for operator recovery"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupDispositionRejectedCustody {
    state: CandidateCleanupExecutionState,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
    intent_event: HashedComputePluginCandidateCleanupStepEvent,
}

/// A failed OS disposition keeps the exact target handle separate from the untouched remainder.
/// It cannot be converted back into a fresh intent or skip to the next object.
#[must_use = "failed disposition must be retried with this exact custody or retained"]
pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupDispositionRetryCustody {
    state: CandidateCleanupExecutionState,
    plan: HashedComputePluginCandidateCleanupExecutionPlan,
    intent_event: HashedComputePluginCandidateCleanupStepEvent,
    pending: PendingCleanupObject,
}

pub(in crate::node_agent_compute_plugin_host) struct CandidateCleanupDispositionFailure {
    phase: CandidateCleanupDispositionFailurePhase,
    error: Error,
    custody: CandidateCleanupDispositionFailureCustody,
}

pub(in crate::node_agent_compute_plugin_host::candidate_cleanup_contract) fn set_candidate_cleanup_delete_disposition(
    intent: DurableCandidateCleanupDeleteIntent,
) -> std::result::Result<PhysicallyDisposedCandidateCleanupObject, CandidateCleanupDispositionFailure>
{
    if let Err(error) = validate_initial_intent_custody(&intent) {
        return Err(intent_failure(error, intent));
    }
    let (sealed, intent_event) = intent.into_parts();
    let (mut state, plan) = sealed.into_parts();
    let Some(pending) = take_next_pending(&mut state) else {
        return Err(rejected_failure(
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_OBJECT_MISSING"),
            CandidateCleanupDispositionRejectedCustody {
                state,
                plan,
                intent_event,
            },
        ));
    };
    state.execution_plan_digest = Some(plan.plan_digest().to_string());
    let retry = CandidateCleanupDispositionRetryCustody {
        state,
        plan,
        intent_event,
        pending,
    };
    execute_retry_custody(retry)
}

pub(in crate::node_agent_compute_plugin_host::candidate_cleanup_contract) fn retry_candidate_cleanup_delete_disposition(
    retry: CandidateCleanupDispositionRetryCustody,
) -> std::result::Result<PhysicallyDisposedCandidateCleanupObject, CandidateCleanupDispositionFailure>
{
    execute_retry_custody(retry)
}

fn execute_retry_custody(
    retry: CandidateCleanupDispositionRetryCustody,
) -> std::result::Result<PhysicallyDisposedCandidateCleanupObject, CandidateCleanupDispositionFailure>
{
    if let Err(error) = validate_retry_custody(&retry) {
        return Err(retry_failure(
            CandidateCleanupDispositionFailurePhase::RejectedBeforeDisposition,
            error,
            retry,
        ));
    }
    let CandidateCleanupDispositionRetryCustody {
        state,
        plan,
        intent_event,
        pending,
    } = retry;
    match pending.set_delete_disposition_exact() {
        Ok(disposition) => Ok(PhysicallyDisposedCandidateCleanupObject {
            state,
            plan,
            intent_event,
            disposition,
            disposition_set_at: Instant::now(),
        }),
        Err((error, pending)) => Err(retry_failure(
            CandidateCleanupDispositionFailurePhase::DispositionFailed,
            error,
            CandidateCleanupDispositionRetryCustody {
                state,
                plan,
                intent_event,
                pending,
            },
        )),
    }
}

fn validate_initial_intent_custody(intent: &DurableCandidateCleanupDeleteIntent) -> Result<()> {
    let sealed = intent.sealed();
    sealed.validate_retained_state()?;
    let expected =
        build_initial_delete_intent(sealed.plan(), intent.event().event().recorded_at_ms())?;
    if expected != *intent.event() {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_INTENT_CHANGED");
    }
    Ok(())
}

fn validate_retry_custody(retry: &CandidateCleanupDispositionRetryCustody) -> Result<()> {
    validate_hashed_execution_plan(&retry.plan)?;
    retry.state.cancellation_guard().ensure_current()?;
    let expected_intent =
        build_initial_delete_intent(&retry.plan, retry.intent_event.event().recorded_at_ms())?;
    let expected_object = retry.plan.objects().first().ok_or_else(|| {
        anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_OBJECT_MISSING")
    })?;
    if expected_intent != retry.intent_event
        || retry.state.execution_plan_digest() != Some(retry.plan.plan_digest())
        || retry.state.completed_step_count() != 0
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DISPOSITION_RETRY_CHANGED");
    }
    retry.pending.validate_expected(expected_object)
}

fn take_next_pending(state: &mut CandidateCleanupExecutionState) -> Option<PendingCleanupObject> {
    if let Some(file) = state.staging_files.pop_front() {
        return Some(PendingCleanupObject::File(file));
    }
    if let Some(file) = state.seal.take() {
        return Some(PendingCleanupObject::File(file));
    }
    if let Some(directory) = state.staging_directories.pop_front() {
        return Some(PendingCleanupObject::Directory(directory));
    }
    if let Some(directory) = state.staging_run.take() {
        return Some(PendingCleanupObject::Directory(directory));
    }
    if let Some(directory) = state.staging_parent.take() {
        return Some(PendingCleanupObject::Directory(directory));
    }
    if let Some(file) = state.download_files.pop_front() {
        return Some(PendingCleanupObject::File(file));
    }
    if let Some(directory) = state.downloads_directory.take() {
        return Some(PendingCleanupObject::Directory(directory));
    }
    state
        .candidate_directory
        .take()
        .map(PendingCleanupObject::Directory)
}

fn intent_failure(
    error: Error,
    intent: DurableCandidateCleanupDeleteIntent,
) -> CandidateCleanupDispositionFailure {
    CandidateCleanupDispositionFailure {
        phase: CandidateCleanupDispositionFailurePhase::RejectedBeforeDisposition,
        error,
        custody: CandidateCleanupDispositionFailureCustody::Intent(intent),
    }
}

fn retry_failure(
    phase: CandidateCleanupDispositionFailurePhase,
    error: Error,
    retry: CandidateCleanupDispositionRetryCustody,
) -> CandidateCleanupDispositionFailure {
    CandidateCleanupDispositionFailure {
        phase,
        error,
        custody: CandidateCleanupDispositionFailureCustody::Retry(retry),
    }
}

fn rejected_failure(
    error: Error,
    rejected: CandidateCleanupDispositionRejectedCustody,
) -> CandidateCleanupDispositionFailure {
    CandidateCleanupDispositionFailure {
        phase: CandidateCleanupDispositionFailurePhase::RejectedBeforeDisposition,
        error,
        custody: CandidateCleanupDispositionFailureCustody::Rejected(rejected),
    }
}

impl PhysicallyDisposedCandidateCleanupObject {
    pub(in crate::node_agent_compute_plugin_host) fn plan(
        &self,
    ) -> &HashedComputePluginCandidateCleanupExecutionPlan {
        &self.plan
    }

    pub(in crate::node_agent_compute_plugin_host) fn intent_event(
        &self,
    ) -> &HashedComputePluginCandidateCleanupStepEvent {
        &self.intent_event
    }

    pub(in crate::node_agent_compute_plugin_host) fn object_binding(
        &self,
    ) -> &ManagedObjectBinding {
        self.disposition.object_binding()
    }

    pub(in crate::node_agent_compute_plugin_host) fn disposition_set_at(&self) -> Instant {
        self.disposition_set_at
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        CandidateCleanupExecutionState,
        HashedComputePluginCandidateCleanupExecutionPlan,
        HashedComputePluginCandidateCleanupStepEvent,
        ManagedDeleteDisposition,
        Instant,
    ) {
        (
            self.state,
            self.plan,
            self.intent_event,
            self.disposition,
            self.disposition_set_at,
        )
    }
}

impl CandidateCleanupDispositionFailure {
    pub(in crate::node_agent_compute_plugin_host) fn phase(
        &self,
    ) -> CandidateCleanupDispositionFailurePhase {
        self.phase
    }

    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, CandidateCleanupDispositionFailureCustody) {
        (self.error, self.custody)
    }
}

impl fmt::Display for CandidateCleanupDispositionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl fmt::Debug for CandidateCleanupDispositionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupDispositionFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl StdError for CandidateCleanupDispositionFailure {}

impl fmt::Debug for PhysicallyDisposedCandidateCleanupObject {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PhysicallyDisposedCandidateCleanupObject")
            .field("plan_digest", &self.plan.plan_digest())
            .field("step_ordinal", &self.intent_event.event().step_ordinal())
            .field("disposition", &self.disposition)
            .field("root_lock", &"<retained>")
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CandidateCleanupDispositionRetryCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupDispositionRetryCustody")
            .field("plan_digest", &self.plan.plan_digest())
            .field("step_ordinal", &self.intent_event.event().step_ordinal())
            .field("target_handle", &"<retained>")
            .field("root_lock", &"<retained>")
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for CandidateCleanupDispositionRejectedCustody {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CandidateCleanupDispositionRejectedCustody")
            .field("plan_digest", &self.plan.plan_digest())
            .field("step_ordinal", &self.intent_event.event().step_ordinal())
            .field(
                "remaining_objects",
                &self.state.topology_objects().ok().map(|v| v.len()),
            )
            .field("root_lock", &"<retained>")
            .finish_non_exhaustive()
    }
}

#[cfg(all(test, windows))]
mod tests {
    use std::{ffi::OsStr, fs, path::Path};

    use uuid::Uuid;

    use super::*;
    use crate::node_agent_compute_plugin_host::{
        candidate_cleanup_contract::{
            restore_hashed_expected_object, ComputePluginCandidateCleanupExpectedObject,
        },
        signed_artifact_verification::jcs_sha256_hex,
    };
    use crate::node_agent_managed_fs::{ManagedParentRelativeObservation, PinnedManagedRoot};

    #[test]
    fn cleanup_strong_disposition_pending_file_must_match_plan_object() {
        let path = std::env::temp_dir().join(format!(
            "elon-cleanup-strong-step-{}",
            Uuid::new_v4().simple()
        ));
        fs::create_dir_all(&path).expect("create test root");
        let root = PinnedManagedRoot::pin(&path, &"a".repeat(64)).expect("pin test root");
        fs::create_dir(path.join("candidate")).expect("create candidate");
        fs::write(path.join("candidate/artifact.bin"), b"").expect("create artifact");
        let directory = root
            .pin_existing_directory_for_cleanup(Path::new("candidate"))
            .expect("pin candidate for cleanup");
        let file = directory
            .open_existing_read_only_cleanup_child(OsStr::new("artifact.bin"))
            .expect("pin artifact for cleanup");
        let binding = file.object_binding().clone();
        let expected_identity = file.identity_digest().to_string();
        let pending = PendingCleanupObject::File(super::super::PendingCleanupFile {
            object_kind: "download_file",
            logical_path: "compute-plugin/candidates/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/downloads/artifact.bin".to_string(),
            content_digest: "b".repeat(64),
            expected_identity_digest: expected_identity.clone(),
            file,
        });
        let object: ComputePluginCandidateCleanupExpectedObject = serde_json::from_value(
            serde_json::json!({
                "schema": "elon.compute_plugin.candidate_cleanup_expected_object.v1",
                "cleanup_id": "cca_strong_step_test",
                "step_ordinal": 0,
                "parent_step_ordinal": 1,
                "topology_depth": 2,
                "object_kind": "file",
                "logical_kind": "download_file",
                "relative_name": "artifact.bin",
                "relative_path": "compute-plugin/candidates/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/downloads/artifact.bin",
                "relative_path_digest": "c".repeat(64),
                "expected_identity_digest": expected_identity,
                "expected_parent_identity_digest": binding.parent_identity_digest(),
                "expected_content_digest": "b".repeat(64),
                "expected_size_bytes": 0
            }),
        )
        .expect("decode expected object");
        let object_digest = jcs_sha256_hex(&object).expect("hash expected object");
        let expected =
            restore_hashed_expected_object(object, object_digest).expect("restore expected object");

        pending
            .validate_expected(&expected)
            .expect("pending object must match plan object");
        let disposition = match pending.set_delete_disposition_exact() {
            Ok(disposition) => disposition,
            Err((error, _retained)) => panic!("set pending disposition: {error:#}"),
        };
        let absence = match disposition
            .observe_parent_relative()
            .expect("observe pending absence")
        {
            ManagedParentRelativeObservation::Absent(absence) => absence,
            _ => panic!("disposed pending file must be absent"),
        };
        drop(absence);
        assert!(matches!(
            directory
                .set_delete_disposition_exact()
                .expect("set directory disposition")
                .observe_parent_relative()
                .expect("observe directory absence"),
            ManagedParentRelativeObservation::Absent(_)
        ));
        drop(root);
        fs::remove_dir(path).expect("remove test root");
    }
}
