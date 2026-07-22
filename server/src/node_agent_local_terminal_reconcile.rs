//! One ordered trust boundary for every durable local CLI completion.

use anyhow::{Context, Result};
use homecli_proto::CliCompletionEnvelope;

use crate::{
    node_agent_local_task_store::LocalTaskRecord,
    node_agent_local_task_supervision::{SupervisionContract, SUPERVISION_PROTOCOL},
    node_agent_supervision_terminal_lease_safety::{
        TerminalLeaseExpectation, VerifiedTerminalLeaseIdentity,
    },
    node_agent_update_recovery_terminal::ExpectedRecovery,
    NodeRuntime,
};

pub(crate) struct LocalTerminalReconciler<'a> {
    runtime: &'a NodeRuntime,
    #[cfg(test)]
    contract_root: Option<std::path::PathBuf>,
    #[cfg(test)]
    receipt_root: Option<std::path::PathBuf>,
    #[cfg(test)]
    fail_after: Option<TerminalWriteBoundary>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TerminalWriteBoundary {
    Receipt,
    LocalTask,
    Journal,
    Recovery,
}

impl<'a> LocalTerminalReconciler<'a> {
    pub(crate) fn from_runtime(runtime: &'a NodeRuntime) -> Self {
        Self {
            runtime,
            #[cfg(test)]
            contract_root: None,
            #[cfg(test)]
            receipt_root: None,
            #[cfg(test)]
            fail_after: None,
        }
    }

    #[cfg(test)]
    fn for_test(
        runtime: &'a NodeRuntime,
        contract_root: std::path::PathBuf,
        receipt_root: std::path::PathBuf,
    ) -> Self {
        Self {
            runtime,
            contract_root: Some(contract_root),
            receipt_root: Some(receipt_root),
            fail_after: None,
        }
    }

    #[cfg(test)]
    fn for_test_with_failure(
        runtime: &'a NodeRuntime,
        contract_root: std::path::PathBuf,
        receipt_root: std::path::PathBuf,
        fail_after: TerminalWriteBoundary,
    ) -> Self {
        Self {
            runtime,
            contract_root: Some(contract_root),
            receipt_root: Some(receipt_root),
            fail_after: Some(fail_after),
        }
    }

    pub(crate) async fn reconcile(&self, completion: &CliCompletionEnvelope) -> Result<()> {
        anyhow::ensure!(
            completion.origin == crate::node_agent_completion_outbox::LOCAL_OFFLINE_ORIGIN,
            "only local_offline completions can use local terminal reconciliation"
        );
        let initial = self
            .runtime
            .local_tasks
            .get(&completion.req_id)?
            .context("durable local completion has no matching local task row")?;
        validate_completion_identity(&initial, completion)?;
        let initial_contract = crate::node_agent_local_task_supervision::load_supervision_contract(
            &self.runtime.task_journal,
            &completion.req_id,
        )?;
        let supervised = is_platform_supervised(&initial);
        let admission_base = if supervised {
            let contract = initial_contract
                .as_ref()
                .context("supervised terminal task is missing its durable supervision contract")?;
            Some(
                crate::node_agent_supervision_terminal_lease_safety::admission_base(
                    &initial,
                    contract,
                    &completion.req_id,
                )?,
            )
        } else {
            None
        };
        let _admission = admission_base
            .as_deref()
            .map(crate::node_agent_supervision_worktree_lease::ResumeAdmissionGuard::acquire)
            .transpose()?;

        // Reload under the cross-process guard so every later preflight and
        // persistence step refers to the same durable task identity.
        let task = self
            .runtime
            .local_tasks
            .get(&completion.req_id)?
            .context("local terminal task disappeared after admission")?;
        validate_completion_identity(&task, completion)?;
        let contract = crate::node_agent_local_task_supervision::load_supervision_contract(
            &self.runtime.task_journal,
            &completion.req_id,
        )?;
        anyhow::ensure!(
            supervised == is_platform_supervised(&task),
            "local task supervision identity drifted during terminal admission"
        );
        let status = crate::node_agent_task_journal_events::completion_terminal_status(
            completion.exit_ok,
            completion.error.as_deref(),
        );
        let verified = if supervised {
            let contract = contract
                .as_ref()
                .context("supervised terminal task is missing its durable supervision contract")?;
            if status == "done" {
                Some(
                    self.verify_completed_identity(&task, contract, completion)
                        .await?,
                )
            } else {
                Some(
                    crate::node_agent_supervision_terminal_lease_safety::verify_terminal_identity(
                        self.runtime,
                        &task,
                        contract,
                        &completion.req_id,
                        TerminalLeaseExpectation::Exact,
                    )
                    .await?,
                )
            }
        } else {
            None
        };

        let expected_recovery = self.expected_recovery(&task, contract.as_ref())?;
        self.runtime.update_recovery.preflight_terminal_completion(
            expected_recovery,
            &completion.req_id,
            &completion.event_id,
            status,
            completion.created_at_ms as u128,
            completion.exit_ok,
            completion.error.as_deref(),
        )?;
        self.runtime
            .local_tasks
            .preflight_completion(completion, verified.as_ref())?;
        self.runtime
            .task_journal
            .preflight_reconciled_finished_with_outcome(
                &completion.req_id,
                &completion.event_id,
                status,
                completion.error.as_deref(),
            )?;
        let finalization = match verified.as_ref() {
            Some(identity) => self.preflight_finalization(identity, completion)?,
            None => crate::node_agent_terminal_finalization::TerminalFinalizationPreflight::not_applicable(),
        };

        // No receipt/local task/journal/recovery write occurs above this line.
        finalization.commit(completion)?;
        self.inject_failure(TerminalWriteBoundary::Receipt)?;
        anyhow::ensure!(
            self.runtime
                .local_tasks
                .reconcile_completion_trusted(completion, verified.as_ref())?,
            "durable local completion could not bind the local terminal row"
        );
        self.inject_failure(TerminalWriteBoundary::LocalTask)?;
        let durable = self
            .runtime
            .local_tasks
            .get(&completion.req_id)?
            .context("local terminal row disappeared after persistence")?;
        assert_durable_terminal(&durable, completion, supervised)?;
        self.runtime
            .task_journal
            .record_reconciled_finished_with_outcome(
                &completion.req_id,
                &completion.event_id,
                status,
                completion.error.as_deref(),
            )?;
        self.inject_failure(TerminalWriteBoundary::Journal)?;
        self.runtime.update_recovery.reconcile_terminal_completion(
            expected_recovery,
            &completion.req_id,
            &completion.event_id,
            status,
            completion.created_at_ms as u128,
            completion.exit_ok,
            completion.error.as_deref(),
        )?;
        self.inject_failure(TerminalWriteBoundary::Recovery)?;
        Ok(())
    }

    #[cfg(test)]
    fn inject_failure(&self, boundary: TerminalWriteBoundary) -> Result<()> {
        anyhow::ensure!(
            self.fail_after != Some(boundary),
            "injected terminal persistence failure after {boundary:?}"
        );
        Ok(())
    }

    #[cfg(not(test))]
    fn inject_failure(&self, _boundary: TerminalWriteBoundary) -> Result<()> {
        Ok(())
    }

    fn expected_recovery(
        &self,
        task: &LocalTaskRecord,
        contract: Option<&SupervisionContract>,
    ) -> Result<ExpectedRecovery> {
        if self
            .runtime
            .update_recovery
            .terminal_receipt_for_task(&task.task_id)?
            .is_some()
        {
            return Ok(ExpectedRecovery::Required);
        }
        let status_requires_receipt = matches!(
            task.status.as_str(),
            "recovering" | "reattaching" | "resume_required"
        );
        let parent_requires_receipt = if contract.is_some_and(|contract| {
            contract.protocol == SUPERVISION_PROTOCOL && contract.task_role == "resume_original"
        }) {
            contract
                .and_then(|contract| contract.parent_task_id.as_deref())
                .map(|parent| self.runtime.local_tasks.get(parent))
                .transpose()?
                .flatten()
                .is_some_and(|parent| {
                    matches!(parent.status.as_str(), "recovering" | "reattaching")
                })
        } else {
            false
        };
        anyhow::ensure!(
            !status_requires_receipt && !parent_requires_receipt,
            "known recovery task has no unique durable recovery receipt"
        );
        Ok(ExpectedRecovery::NotApplicable)
    }

    fn preflight_finalization<'b>(
        &self,
        identity: &'b VerifiedTerminalLeaseIdentity,
        completion: &CliCompletionEnvelope,
    ) -> Result<crate::node_agent_terminal_finalization::TerminalFinalizationPreflight<'b>> {
        #[cfg(test)]
        if let (Some(contract_root), Some(receipt_root)) =
            (self.contract_root.as_deref(), self.receipt_root.as_deref())
        {
            return crate::node_agent_terminal_finalization::preflight_with_roots_for_test(
                identity,
                completion,
                contract_root,
                receipt_root,
            );
        }
        crate::node_agent_terminal_finalization::preflight(identity, completion)
    }

    async fn verify_completed_identity(
        &self,
        task: &LocalTaskRecord,
        contract: &SupervisionContract,
        completion: &CliCompletionEnvelope,
    ) -> Result<VerifiedTerminalLeaseIdentity> {
        #[cfg(test)]
        if let (Some(contract_root), Some(receipt_root)) =
            (self.contract_root.as_deref(), self.receipt_root.as_deref())
        {
            return crate::node_agent_terminal_finalization::verify_completed_identity_for_test(
                self.runtime,
                task,
                contract,
                completion,
                contract_root,
                receipt_root,
            )
            .await;
        }
        crate::node_agent_terminal_finalization::verify_completed_identity(
            self.runtime,
            task,
            contract,
            completion,
        )
        .await
    }
}

fn validate_completion_identity(
    task: &LocalTaskRecord,
    completion: &CliCompletionEnvelope,
) -> Result<()> {
    let producer = completion
        .producer_identity
        .as_ref()
        .context("local completion is missing producer identity")?;
    let project = completion
        .project_context
        .as_ref()
        .context("local completion is missing project context")?;
    anyhow::ensure!(
        producer.owner_user_id == task.owner_user_id
            && producer.agent_id == task.agent_id
            && producer.install_id == task.install_id,
        "local completion producer identity drifted"
    );
    anyhow::ensure!(
        crate::node_agent_full_access::project_ids_equivalent(
            &project.project_id,
            &task.project_id
        ) && project.conversation_id == task.conversation_id,
        "local completion project or conversation identity drifted"
    );
    Ok(())
}

fn assert_durable_terminal(
    task: &LocalTaskRecord,
    completion: &CliCompletionEnvelope,
    supervised: bool,
) -> Result<()> {
    let status = crate::node_agent_task_journal_events::completion_terminal_status(
        completion.exit_ok,
        completion.error.as_deref(),
    );
    let outcome =
        (!completion.final_output.trim().is_empty()).then_some(completion.final_output.as_str());
    anyhow::ensure!(
        task.completion_event_id.as_deref() == Some(completion.event_id.as_str())
            && task.status == status
            && task.error.as_deref() == completion.error.as_deref()
            && task.final_reply.as_deref() == outcome
            && task.finished_at_ms == Some(completion.created_at_ms.min(i64::MAX as u64) as i64),
        "local terminal row did not preserve event, status, outcome, and finished identity"
    );
    if supervised {
        anyhow::ensure!(
            task.workspace_status
                .as_ref()
                .and_then(|value| value.get("terminal_snapshot_status"))
                .and_then(serde_json::Value::as_str)
                == Some("trusted"),
            "supervised terminal row lacks a trusted workspace snapshot"
        );
    }
    Ok(())
}

fn is_platform_supervised(task: &LocalTaskRecord) -> bool {
    task.workspace_status
        .as_ref()
        .and_then(|value| value.get("platform_provenance"))
        .and_then(serde_json::Value::as_str)
        == Some("elon.conversation_worktree.v1")
}

#[cfg(test)]
#[path = "node_agent_local_terminal_reconcile_tests.rs"]
mod tests;
