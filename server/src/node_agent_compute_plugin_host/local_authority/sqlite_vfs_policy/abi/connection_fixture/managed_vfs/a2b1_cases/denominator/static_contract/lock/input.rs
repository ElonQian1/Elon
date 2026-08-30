use super::{
    super::{
        model::{CustodyState, DecisionStage, FailureClass, LockEffect},
        source::{witness, ProductionOwner, SourceWitness},
    },
    builder::Builder,
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
    for (cell, needle) in [
        (
            "range-overflow",
            "NODE_MANAGED_SQLITE_SHM_LOCK_RANGE_OVERFLOW",
        ),
        (
            "end-past-eight",
            "NODE_MANAGED_SQLITE_SHM_LOCK_RANGE_INVALID",
        ),
    ] {
        add_request_rejection(builder, &request_gate, &prefix, cell, needle);
    }
    if action.is_shared() {
        add_request_rejection(
            builder,
            &request_gate,
            &prefix,
            "shared-multi-slot",
            "NODE_MANAGED_SQLITE_SHM_SHARED_LOCK_NOT_SINGLE_SLOT",
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
    cell: &str,
    needle: &'static str,
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

fn abi_witness(needle: &'static str) -> SourceWitness {
    witness(
        ProductionOwner::AbiIoShm,
        "unsafe extern \"C\" fn lock",
        needle,
        1,
    )
}
