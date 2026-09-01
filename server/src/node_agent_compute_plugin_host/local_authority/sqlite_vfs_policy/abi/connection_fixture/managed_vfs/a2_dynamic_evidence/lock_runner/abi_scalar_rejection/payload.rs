//! Canonical q10 payload built only from source bindings and consumed runtime observations.

use anyhow::anyhow;
use rusqlite::ffi;

use super::super::super::super::{
    connection::ManagedTestShmLockAbiLedgerObservation,
    ManagedSqliteTestVfsRouteCustodySnapshot, ManagedSqliteTestVfsRoutePhase,
};
use super::super::super::child::lock_abi_scalar_rejection;
use super::{
    exact_selector, raw_tuple, validate_binding, LockRunnerAbiScalarRejectionBindingV1,
    LockRunnerAbiScalarValidityV1,
};

pub(super) fn encode_payload(
    binding: LockRunnerAbiScalarRejectionBindingV1,
    registration_id: u64,
    route_ordinal: u64,
    observation: &ManagedTestShmLockAbiLedgerObservation,
    route_no_entry: [u64; 18],
    target_before: bool,
    target_after: bool,
    route_before: ManagedSqliteTestVfsRouteCustodySnapshot,
    route_after: ManagedSqliteTestVfsRouteCustodySnapshot,
    registration_values: [u64; 4],
    terminal_values: [u64; 4],
) -> String {
    let callback = observation.callback();
    let abi = observation.abi();
    let mut values = binding_values(binding);
    values.extend([
        registration_id,
        route_ordinal,
        callback.offset() as u64,
        callback.count() as u64,
        callback.raw_flags() as u64,
        callback.result_code() as u64,
        u64::from(callback.before().methods_installed),
        u64::from(callback.before().state_installed),
        u64::from(callback.before().methods_exact),
        u64::from(callback.before().state_type_exact),
        u64::from(callback.after().methods_installed),
        u64::from(callback.after().state_installed),
        u64::from(callback.after().methods_exact),
        u64::from(callback.after().state_type_exact),
        abi.observation_id(),
        abi.offset() as u64,
        abi.count() as u64,
        abi.flags() as u64,
        abi.entry_count(),
        abi.scalar_rejection_count(),
        u64::from(abi.offset_valid()),
        u64::from(abi.count_valid()),
        u64::from(abi.flags_valid()),
        abi.run_code_entry_count(),
        abi.return_count(),
        abi.result_code() as u64,
    ]);
    values.extend(route_no_entry);
    values.extend([u64::from(target_before), u64::from(target_after)]);
    values.extend(route_values(route_before));
    values.extend(route_values(route_after));
    values.extend(registration_values);
    values.extend(terminal_values);
    debug_assert_eq!(values.len(), lock_abi_scalar_rejection::REPORT_VALUE_COUNT);
    format!(
        "{},{},{}",
        lock_abi_scalar_rejection::REPORT_VERSION,
        exact_selector(binding),
        values
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    )
}

pub(super) fn validate_payload(
    payload: &str,
    binding: LockRunnerAbiScalarRejectionBindingV1,
) -> anyhow::Result<u64> {
    validate_binding(binding)?;
    let mut fields = payload.split(',');
    let selector = exact_selector(binding);
    if fields.next() != Some(lock_abi_scalar_rejection::REPORT_VERSION)
        || fields.next() != Some(selector.as_str())
    {
        return Err(anyhow!("q10 Lock ABI-scalar payload identity mismatch"));
    }
    let values = fields
        .map(parse_canonical_u64)
        .collect::<anyhow::Result<Vec<_>>>()?;
    if values.len() != lock_abi_scalar_rejection::REPORT_VALUE_COUNT
        || values[..23] != binding_values(binding)
    {
        return Err(anyhow!("q10 Lock ABI-scalar payload program binding mismatch"));
    }
    let (offset, count, raw_flags) = raw_tuple(binding);
    let callback = [
        offset as u64,
        count as u64,
        raw_flags as u64,
        ffi::SQLITE_IOERR_SHMLOCK as u64,
    ];
    let validity = scalar_validity(binding);
    let abi = [
        offset as u64,
        count as u64,
        raw_flags as u64,
        1,
        1,
        validity[0],
        validity[1],
        validity[2],
        0,
        1,
        ffi::SQLITE_IOERR_SHMLOCK as u64,
    ];
    let route_before = &values[69..75];
    let route_after = &values[75..81];
    if values[23] == 0
        || values[24] == 0
        || values[25..29] != callback
        || values[29..37] != [1; 8]
        || values[37] == 0
        || values[38..49] != abi
        || values[49..67]
            != [1, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        || values[67..69] != [0, 0]
        || route_before != route_after
        || route_before[0] != phase_tag(ManagedSqliteTestVfsRoutePhase::Active)
        || route_before[1] != 1
        || route_before[2] != 0
        || route_before[3] != 0
        || route_before[4] != 0
        || route_before[5] != 1
        || values[81..85] != [1; 4]
        || values[85..89] != [1; 4]
    {
        return Err(anyhow!(
            "q10 Lock ABI-scalar payload installed callback receipt mismatch"
        ));
    }
    Ok(values[23])
}

fn binding_values(binding: LockRunnerAbiScalarRejectionBindingV1) -> Vec<u64> {
    let mut values = vec![
        binding.offset.tag(),
        binding.count.tag(),
        binding.flags.tag(),
    ];
    for digest in [
        binding.normalized_descriptor_sha256,
        binding.case_key_sha256,
        binding.full_record_sha256,
        binding.plan_sha256,
        binding.implementation_sha256,
    ] {
        for chunk in digest.chunks_exact(8) {
            values.push(u64::from_le_bytes(chunk.try_into().expect("digest chunk")));
        }
    }
    values
}

fn scalar_validity(binding: LockRunnerAbiScalarRejectionBindingV1) -> [u64; 3] {
    [binding.offset, binding.count, binding.flags].map(|value| {
        u64::from(matches!(value, LockRunnerAbiScalarValidityV1::Valid))
    })
}

fn route_values(snapshot: ManagedSqliteTestVfsRouteCustodySnapshot) -> [u64; 6] {
    [
        phase_tag(snapshot.phase()),
        u64::from(snapshot.connection_owner()),
        u64::from(snapshot.main_file_lock_owner_lease()),
        u64::from(snapshot.shm_lease()),
        snapshot.callbacks_in_flight() as u64,
        u64::from(snapshot.access_callback_allowed()),
    ]
}

const fn phase_tag(phase: ManagedSqliteTestVfsRoutePhase) -> u64 {
    match phase {
        ManagedSqliteTestVfsRoutePhase::PendingMain => 1,
        ManagedSqliteTestVfsRoutePhase::Opening => 2,
        ManagedSqliteTestVfsRoutePhase::Active => 3,
        ManagedSqliteTestVfsRoutePhase::Closing => 4,
        ManagedSqliteTestVfsRoutePhase::AwaitingRouteRetirement => 5,
        ManagedSqliteTestVfsRoutePhase::Retired => 6,
        ManagedSqliteTestVfsRoutePhase::TerminalQuarantine => 7,
    }
}

fn parse_canonical_u64(value: &str) -> anyhow::Result<u64> {
    let parsed = value.parse::<u64>()?;
    if parsed.to_string() != value {
        return Err(anyhow!(
            "q10 Lock ABI-scalar payload scalar is not canonical"
        ));
    }
    Ok(parsed)
}
