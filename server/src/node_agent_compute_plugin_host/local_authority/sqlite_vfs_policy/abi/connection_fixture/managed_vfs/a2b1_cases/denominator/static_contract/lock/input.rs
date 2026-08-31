use super::super::terminal_descriptor::{
    FaultSeamV1, LockAbiScalarV1, LockManagedStimulusV1, LockOperationV1, LockPrestateV1,
    ObserverV1, OccurrenceV1, PhaseV1, SourceSiteV1, StimulusV1, TimingV1, ValidityV1,
};
use super::{
    super::{
        model::{CustodyState, DecisionStage, FailureClass, LockEffect},
        source::{witness, ProductionOwner, SourceWitness},
    },
    builder::Builder,
    dynamic::{SeedV1, TerminalPathV1},
    outcome,
    range::{self, Action, RangeCell},
};

mod raw;

#[derive(Debug, Clone)]
pub(super) struct ValidRequest {
    pub(super) action: Action,
    pub(super) range: RangeCell,
    pub(super) node: String,
    pub(super) prefix: String,
}

impl ValidRequest {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn descriptor(
        &self,
        source_site: SourceSiteV1,
        stimulus: LockManagedStimulusV1,
        prestate: LockPrestateV1,
        operation: LockOperationV1,
        phase: PhaseV1,
        timing: TimingV1,
        fault_seam: FaultSeamV1,
    ) -> SeedV1 {
        SeedV1::managed(
            self.action,
            self.range,
            source_site,
            stimulus,
            prestate,
            operation,
            phase,
            timing,
            fault_seam,
        )
    }
}

pub(super) fn build(builder: &mut Builder) -> (String, Vec<ValidRequest>) {
    let root = builder.decision(
        "lock.root.x-shm-lock",
        witness(
            ProductionOwner::SqliteVfsAbiTable,
            "static INERT_IO_METHODS",
            "xShmLock: Some(io_shm::lock)",
            1,
        ),
    );
    let abi = builder.decision(
        "lock.abi.scalar-validation",
        abi_witness("let (Ok(offset), Some(count), Some(action)) ="),
    );
    builder.edge(
        &root,
        &abi,
        DecisionStage::AbiValidation,
        "dispatch_x_shm_lock",
    );

    for offset_valid in [false, true] {
        for count_valid in [false, true] {
            for flags_valid in [false, true] {
                if offset_valid && count_valid && flags_valid {
                    continue;
                }
                let cell = format!(
                    "offset-{}.count-{}.flags-{}",
                    validity(offset_valid),
                    validity(count_valid),
                    validity(flags_valid)
                );
                let expected = outcome::unavailable("AbiValidation");
                outcome::direct(
                    builder,
                    &abi,
                    &format!("lock.abi.invalid.{cell}"),
                    DecisionStage::AbiValidation,
                    &cell,
                    expected,
                    SeedV1::early(
                        SourceSiteV1::LockAbiBoundary,
                        StimulusV1::LockAbi(LockAbiScalarV1 {
                            offset: typed_validity(offset_valid),
                            count: typed_validity(count_valid),
                            flags: typed_validity(flags_valid),
                        }),
                        LockOperationV1::AbiValidation,
                        PhaseV1::AbiValidation,
                        TimingV1::BeforeCall,
                        OccurrenceV1::Natural,
                        FaultSeamV1::AbiBoundary,
                        ObserverV1::LockCallbackAndSnapshot,
                    )
                    .terminal(TerminalPathV1::Direct),
                    abi_witness("return result_codes::SHM_LOCK_UNAVAILABLE;"),
                );
            }
        }
    }

    let raw = builder.decision(
        "lock.raw.admission",
        witness(
            ProductionOwner::AbiRawState,
            "pub(super) unsafe fn with_installed_state",
            "let envelope = unsafe { installed_envelope(file)? };",
            1,
        ),
    );
    builder.edge(&abi, &raw, DecisionStage::RawAdmission, "all_scalars_valid");
    let file_present = raw::build(builder, &raw);
    let action_gate = builder.decision(
        "lock.adapter.action-projection",
        witness(
            ProductionOwner::RegistryAbiFile,
            "fn shm_lock",
            "let action = match action",
            1,
        ),
    );
    builder.edge(
        &file_present,
        &action_gate,
        DecisionStage::Adapter,
        "file_payload_present",
    );

    let mut requests = Vec::new();
    for action in [
        Action::LockShared,
        Action::LockExclusive,
        Action::UnlockShared,
        Action::UnlockExclusive,
    ] {
        add_action(builder, &action_gate, action, &mut requests);
    }
    (root, requests)
}

fn add_action(
    builder: &mut Builder,
    action_gate: &str,
    action: Action,
    requests: &mut Vec<ValidRequest>,
) {
    let prefix = format!("lock.request.{}", action.label());
    let request_gate = builder.decision(
        format!("{prefix}.validation"),
        witness(
            ProductionOwner::ManagedTypes,
            "pub(crate) fn new",
            "let end = first",
            1,
        ),
    );
    builder.edge(
        action_gate,
        &request_gate,
        DecisionStage::ManagedRequest,
        action.label(),
    );
    for (cell, needle, stimulus) in [
        (
            "range-overflow",
            "NODE_MANAGED_SQLITE_SHM_LOCK_RANGE_OVERFLOW",
            LockManagedStimulusV1::RangeOverflow,
        ),
        (
            "end-past-eight",
            "NODE_MANAGED_SQLITE_SHM_LOCK_RANGE_INVALID",
            LockManagedStimulusV1::EndPastEight,
        ),
    ] {
        add_request_rejection(
            builder,
            &request_gate,
            &prefix,
            action,
            cell,
            needle,
            stimulus,
        );
    }
    if action.is_shared() {
        add_request_rejection(
            builder,
            &request_gate,
            &prefix,
            action,
            "shared-multi-slot",
            "NODE_MANAGED_SQLITE_SHM_SHARED_LOCK_NOT_SINGLE_SLOT",
            LockManagedStimulusV1::SharedMultiSlot,
        );
    }
    for range in range::representatives(action) {
        let range_label = range.label();
        let request_prefix = format!("{prefix}.{range_label}");
        let node = builder.continuation(
            format!("{request_prefix}.valid"),
            "Lock managed operation expansion",
            witness(
                ProductionOwner::ManagedTypes,
                "fn mask",
                "(high - low) as u8",
                1,
            ),
        );
        builder.edge(
            &request_gate,
            &node,
            DecisionStage::ManagedRequest,
            format!("valid-orbit.{range_label}"),
        );
        requests.push(ValidRequest {
            action,
            range,
            node,
            prefix: request_prefix,
        });
    }
}

fn add_request_rejection(
    builder: &mut Builder,
    request_gate: &str,
    prefix: &str,
    action: Action,
    cell: &str,
    needle: &'static str,
    stimulus: LockManagedStimulusV1,
) {
    let mut expected = outcome::unavailable("RequestValidation");
    expected.failure = FailureClass::ProtocolViolation;
    expected.raw_slots = CustodyState::Unchanged;
    expected.lock_effect = LockEffect::Unchanged;
    expected.file = CustodyState::Unchanged;
    outcome::managed_direct(
        builder,
        request_gate,
        &format!("{prefix}.rejected.{cell}"),
        DecisionStage::ManagedRequest,
        cell,
        expected,
        SeedV1::request_rejection(action, stimulus),
        witness(
            ProductionOwner::ManagedTypes,
            "pub(crate) fn new",
            needle,
            1,
        ),
    );
}

fn validity(value: bool) -> &'static str {
    if value {
        "valid"
    } else {
        "invalid"
    }
}

const fn typed_validity(value: bool) -> ValidityV1 {
    if value {
        ValidityV1::Valid
    } else {
        ValidityV1::Invalid
    }
}

fn abi_witness(needle: &'static str) -> SourceWitness {
    witness(
        ProductionOwner::AbiIoShm,
        "unsafe extern \"C\" fn lock",
        needle,
        1,
    )
}
