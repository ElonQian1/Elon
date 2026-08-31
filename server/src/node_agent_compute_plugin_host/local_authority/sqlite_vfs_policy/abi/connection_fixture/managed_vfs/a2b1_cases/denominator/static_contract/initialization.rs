//! Ordered production node-initialization graph shared by Map and the two Lock acquire actions.
//!
//! Test fault selectors are intentionally outside this source denominator. Every production
//! branch is connected in source order; callers only add operation-specific projections.

mod dms;
mod open;

use super::{
    model::{
        ContractEdge, ContractNode, CustodyState, Decision, DecisionStage, DmsLockCustody,
        ExclusionProof, FailureClass, MutationState, NodeKind, TerminalDisposition,
    },
    source::{witness, ProductionOwner, SourceWitness},
    terminal_descriptor::{
        InitializationProfileV1, InitializationStimulusV1, OccurrenceV1, PhaseV1, TimingV1,
    },
};

#[derive(Debug, Default)]
pub(super) struct InitializationExpansion {
    pub(super) entry: String,
    pub(super) successes: Vec<InitializationSuccess>,
    pub(super) failures: Vec<InitializationFailure>,
    pub(super) nodes: Vec<ContractNode>,
    pub(super) edges: Vec<ContractEdge>,
    pub(super) leaf_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct InitializationSuccess {
    pub(super) node: String,
    pub(super) label: &'static str,
    pub(super) profile: InitializationProfileV1,
    pub(super) mutation: MutationState,
    pub(super) dms_lock: DmsLockCustody,
    pub(super) native_lock: u16,
    pub(super) native_unlock: u16,
}

#[derive(Debug, Clone)]
pub(super) struct InitializationFailure {
    pub(super) node: String,
    pub(super) projection_prefix: String,
    pub(super) phase: &'static str,
    pub(super) typed_phase: PhaseV1,
    pub(super) stimulus: InitializationStimulusV1,
    pub(super) timing: TimingV1,
    pub(super) occurrence: OccurrenceV1,
    pub(super) class: FailureClass,
    pub(super) mutation: MutationState,
    pub(super) lock_uncertain: bool,
    pub(super) disposition: TerminalDisposition,
    pub(super) file: CustodyState,
    pub(super) dms_lock: DmsLockCustody,
    pub(super) native_lock: u16,
    pub(super) native_unlock: u16,
}

#[derive(Debug, Clone, Copy)]
struct FailureShape {
    phase: PhaseV1,
    stimulus: InitializationStimulusV1,
    timing: TimingV1,
    occurrence: OccurrenceV1,
    class: FailureClass,
    mutation: MutationState,
    lock_uncertain: bool,
    disposition: TerminalDisposition,
    file: CustodyState,
    dms_lock: DmsLockCustody,
    native_lock: u16,
    native_unlock: u16,
}

impl FailureShape {
    fn cleanup_rewrite(self) -> Self {
        Self {
            phase: PhaseV1::FileClose,
            stimulus: InitializationStimulusV1 {
                cleanup_rewrite: true,
                ..self.stimulus
            },
            timing: TimingV1::Cleanup,
            class: FailureClass::OutcomeUncertainPoisoned,
            disposition: TerminalDisposition::CleanupRewritten,
            file: CustodyState::Quarantined,
            ..self
        }
    }
}

pub(super) fn build(prefix: &str) -> InitializationExpansion {
    let entry = format!("{prefix}.entry.state-poison");
    let mut builder = InitBuilder::new(
        prefix,
        &entry,
        init("fn ensure_node", "if let Some(poison) = state.poisoned"),
    );
    let dominated = builder.excluded(
        "state-poisoned-after-outer-check",
        ExclusionProof::ControlFlow(
            "Map/Lock checks the same state.poisoned field while holding the same coordinator MutexGuard and cannot poison it before ensure_node",
        ),
        init("fn ensure_node", "if let Some(poison) = state.poisoned"),
    );
    builder.edge(&entry, &dominated, "state_poisoned_after_outer_check");

    let node_presence = builder.decision(
        "node-presence",
        init("fn ensure_node", "let opened_now = state.node.is_none()"),
    );
    builder.edge(&entry, &node_presence, "state_not_poisoned");
    builder.success(
        &node_presence,
        "node-live",
        InitializationProfileV1::NodeLive,
        "node_already_live",
        false,
        MutationState::None,
        DmsLockCustody::ExistingShared,
        0,
        0,
    );
    open::build(&mut builder, &node_presence);
    builder.finish()
}

struct InitBuilder {
    prefix: String,
    expansion: InitializationExpansion,
}

impl InitBuilder {
    fn new(prefix: &str, entry: &str, source: SourceWitness) -> Self {
        let mut expansion = InitializationExpansion {
            entry: entry.to_owned(),
            ..InitializationExpansion::default()
        };
        expansion
            .nodes
            .push(contract_node(entry, NodeKind::Decision, source));
        Self {
            prefix: prefix.to_owned(),
            expansion,
        }
    }

    fn finish(self) -> InitializationExpansion {
        self.expansion
    }

    fn decision(&mut self, suffix: &str, source: SourceWitness) -> String {
        self.node(suffix, NodeKind::Decision, source)
    }

    fn continuation(&mut self, suffix: &str, owner: &'static str, source: SourceWitness) -> String {
        self.node(
            suffix,
            NodeKind::Continuation {
                expansion_owner: owner,
            },
            source,
        )
    }

    fn excluded(&mut self, suffix: &str, proof: ExclusionProof, source: SourceWitness) -> String {
        let id = self.id(&format!("excluded.{suffix}"));
        self.expansion.leaf_ids.push(id.clone());
        self.expansion.nodes.push(contract_node(
            &id,
            NodeKind::Excluded {
                leaf_id: id.clone(),
                proof,
            },
            source,
        ));
        id
    }

    fn edge(&mut self, from: &str, to: &str, branch: impl Into<String>) {
        self.expansion.edges.push(ContractEdge {
            from: from.to_owned(),
            to: to.to_owned(),
            decision: Decision::new(DecisionStage::Initialization, branch),
        });
    }

    fn failure(
        &mut self,
        from: &str,
        cell: &str,
        branch: &str,
        source: SourceWitness,
        shape: FailureShape,
    ) {
        let node = self.continuation(
            &format!("failure.{cell}"),
            "caller Map/Lock registry SHM failure projection",
            source,
        );
        self.edge(from, &node, branch);
        self.expansion.failures.push(InitializationFailure {
            node,
            projection_prefix: self.id(&format!("projection.{cell}")),
            phase: shape.phase.static_name(),
            typed_phase: shape.phase,
            stimulus: shape.stimulus,
            timing: shape.timing,
            occurrence: shape.occurrence,
            class: shape.class,
            mutation: shape.mutation,
            lock_uncertain: shape.lock_uncertain,
            disposition: shape.disposition,
            file: shape.file,
            dms_lock: shape.dms_lock,
            native_lock: shape.native_lock,
            native_unlock: shape.native_unlock,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn success(
        &mut self,
        from: &str,
        label: &'static str,
        profile: InitializationProfileV1,
        branch: &str,
        opened_now: bool,
        mutation: MutationState,
        dms_lock: DmsLockCustody,
        native_lock: u16,
        native_unlock: u16,
    ) {
        assert_eq!(
            label,
            profile.static_label(),
            "typed initialization profile drift"
        );
        let installed = if opened_now {
            let returned = self.continuation(
                &format!("success.{label}.open-node-returned"),
                "ensure_node state installation",
                init("fn open_node", "Ok(node)"),
            );
            self.edge(from, &returned, branch);
            let assigned = self.continuation(
                &format!("success.{label}.state-assigned"),
                "ensure_node finalization",
                init("fn ensure_node", "state.node = Some(node)"),
            );
            self.edge(&returned, &assigned, "assign_opened_node");
            assigned
        } else {
            from.to_owned()
        };
        let post_check = self.decision(
            &format!("success.{label}.post-open-check"),
            init("fn ensure_node", "if state.node.is_none()"),
        );
        self.edge(
            &installed,
            &post_check,
            if opened_now {
                "evaluate_node_after_open"
            } else {
                branch
            },
        );
        let missing = self.excluded(
            &format!("success.{label}.node-missing-after-open"),
            ExclusionProof::ControlFlow(if opened_now {
                "open_node returned Ok and ensure_node assigned that exact node immediately before this check"
            } else {
                "opened_now is false only when state.node was already Some under the same exclusive state borrow"
            }),
            init(
                "fn ensure_node",
                "NODE_MANAGED_SQLITE_SHM_NODE_MISSING_AFTER_OPEN",
            ),
        );
        self.edge(&post_check, &missing, "node_missing_after_open");
        let final_match = self.decision(
            &format!("success.{label}.final-match"),
            init("fn ensure_node", "match state.node.as_mut()"),
        );
        self.edge(&post_check, &final_match, "node_present_after_open");
        let final_none = self.excluded(
            &format!("success.{label}.final-match-none"),
            ExclusionProof::ControlFlow(
                "the immediately preceding state.node.is_none check rejected None under the same mutable state borrow",
            ),
            init("fn ensure_node", "None => Err(self.poisoned_failure())"),
        );
        self.edge(&final_match, &final_none, "final_match_none");
        let node = self.continuation(
            &format!("success.{label}"),
            "caller post-initialization operation",
            init("fn ensure_node", "Ok((node, initialization_mutated))"),
        );
        self.edge(&final_match, &node, "final_match_some");
        self.expansion.successes.push(InitializationSuccess {
            node,
            label,
            profile,
            mutation,
            dms_lock,
            native_lock,
            native_unlock,
        });
    }

    fn node(&mut self, suffix: &str, kind: NodeKind, source: SourceWitness) -> String {
        let id = self.id(suffix);
        self.expansion.nodes.push(contract_node(&id, kind, source));
        id
    }

    fn id(&self, suffix: &str) -> String {
        format!("{}.{}", self.prefix, suffix)
    }
}

fn contract_node(id: &str, kind: NodeKind, source: SourceWitness) -> ContractNode {
    ContractNode {
        id: id.to_owned(),
        kind,
        witness: Some(source),
    }
}

fn init(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::ManagedInitialization, symbol, needle, 1)
}

fn failure_custody(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::ManagedFailureCustody, symbol, needle, 1)
}

fn namespace(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::ManagedNamespace, symbol, needle, 1)
}

fn namespace_types(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::ManagedNamespaceTypes, symbol, needle, 1)
}

fn shm_root(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::ManagedShmRoot, symbol, needle, 1)
}

fn namespace_close(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::ManagedNamespaceClose, symbol, needle, 1)
}

fn windows_locking(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::WindowsLocking, symbol, needle, 1)
}
