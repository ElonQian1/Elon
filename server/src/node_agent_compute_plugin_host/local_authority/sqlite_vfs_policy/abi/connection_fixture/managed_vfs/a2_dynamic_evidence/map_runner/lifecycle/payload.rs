//! Canonical q3 encoding and independent parent validation for positive Map receipts.

use anyhow::anyhow;
use rusqlite::ffi;
use sha2::{Digest, Sha256};

use crate::node_agent_managed_fs::{
    ManagedSqliteShmMapMode, ManagedSqliteShmTestDmsCustody,
    ManagedSqliteShmTestMapDmsPath, ManagedSqliteShmTestMapPath,
    ManagedSqliteShmTestMapReceipt, ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::connection::ManagedTestShmMapCallbackObservation;
use super::super::super::child::map_lifecycle::{selector, REPORT_VALUE_COUNT, REPORT_VERSION};
use super::fixture::{dms_tag, file_size_before, logical_end, snapshot_values};
use super::{
    mode_tag, path_tag, MapRunnerLifecycleBindingV1, MapRunnerLifecyclePathV1, REGION_SIZE,
};

const NATIVE_RECEIPT_END: usize = 100;

pub(in super::super) struct ValidatedLifecyclePayloadV1 {
    pub(in super::super) registration_id: u64,
    pub(in super::super) native_receipt_sha256: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode(
    binding: MapRunnerLifecycleBindingV1,
    registration_id: u64,
    route_ordinal: u64,
    runtime_generation: u64,
    shm_connection_id: u64,
    callback: ManagedTestShmMapCallbackObservation,
    before: ManagedSqliteShmTestTargetSnapshot,
    after: ManagedSqliteShmTestTargetSnapshot,
    receipt: ManagedSqliteShmTestMapReceipt,
    relation: super::fixture::ValidatedPointerRelationV1,
    registration: [u64; 4],
    route: [u64; 3],
    terminal: [u64; 4],
) -> String {
    let mut values = binding_values(binding);
    values.extend([
        registration_id,
        route_ordinal,
        runtime_generation,
        shm_connection_id,
        u64::from(binding.path.region()),
        u64::from(REGION_SIZE),
        mode_tag(binding.path),
    ]);
    values.extend([
        callback.region() as u64,
        callback.region_size() as u64,
        callback.raw_extend() as u64,
        callback.result_code() as u64,
        u64::from(callback.output_was_cleared()),
        u64::from(relation.output_present),
        u64::from(callback.before().methods_installed),
        u64::from(callback.before().state_installed),
        u64::from(callback.after().methods_installed),
        u64::from(callback.after().state_installed),
    ]);
    values.extend(snapshot_values(before));
    values.extend(snapshot_values(after));
    values.extend(map_receipt_values(receipt, relation));
    values.extend(registration);
    values.extend(route);
    values.extend(terminal);
    debug_assert_eq!(values.len(), REPORT_VALUE_COUNT);
    format!(
        "{REPORT_VERSION},{},{}",
        exact_selector(binding.path),
        values
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    )
}

pub(in super::super) fn validate_payload(
    payload: &str,
    binding: MapRunnerLifecycleBindingV1,
) -> anyhow::Result<ValidatedLifecyclePayloadV1> {
    super::validate_binding(binding)?;
    let mut fields = payload.split(',');
    if fields.next() != Some(REPORT_VERSION)
        || fields.next() != Some(exact_selector(binding.path))
    {
        return Err(anyhow!("Map lifecycle payload identity mismatch"));
    }
    let values = fields
        .map(parse_canonical_u64)
        .collect::<anyhow::Result<Vec<_>>>()?;
    if values.len() != REPORT_VALUE_COUNT || values[..22] != binding_values(binding) {
        return Err(anyhow!("Map lifecycle payload program binding mismatch"));
    }
    if values[22..26].contains(&0)
        || values[22..29]
            != [
                values[22],
                values[23],
                values[24],
                values[25],
                u64::from(binding.path.region()),
                u64::from(REGION_SIZE),
                mode_tag(binding.path),
            ]
        || values[29..39]
            != [
                u64::from(binding.path.region()),
                u64::from(REGION_SIZE),
                binding.path.raw_extend() as u64,
                ffi::SQLITE_OK as u64,
                u64::from(!binding.path.is_mapped()),
                u64::from(binding.path.is_mapped()),
                1,
                1,
                1,
                1,
            ]
    {
        return Err(anyhow!(
            "Map lifecycle payload installed-ABI binding mismatch"
        ));
    }
    if values[39..53] != expected_snapshot_values(binding.path, false)
        || values[53..67] != expected_snapshot_values(binding.path, true)
        || values[67..100]
            != expected_map_receipt_values(binding.path, values[24], values[25])
        || values[100..104] != [1, 1, 1, 1]
        || values[104..107] != [1, 1, 3]
        || values[107..111] != [1, 1, 1, 1]
    {
        return Err(anyhow!("Map lifecycle payload native receipt mismatch"));
    }
    Ok(ValidatedLifecyclePayloadV1 {
        registration_id: values[22],
        native_receipt_sha256: digest_native_receipt(&values),
    })
}

fn binding_values(binding: MapRunnerLifecycleBindingV1) -> Vec<u64> {
    let mut values = vec![path_tag(binding.path), mode_tag(binding.path)];
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

fn map_receipt_values(
    value: ManagedSqliteShmTestMapReceipt,
    relation: super::fixture::ValidatedPointerRelationV1,
) -> [u64; 33] {
    [
        value.runtime_generation,
        value.shm_connection_id,
        u64::from(value.expectation.region),
        u64::from(value.expectation.region_size),
        managed_mode_tag(value.expectation.mode),
        managed_path_tag(value.expectation.path),
        managed_dms_path_tag(value.expectation.dms_path),
        u64::from(value.managed_attempts),
        u64::from(value.created_first_shared),
        u64::from(value.node_live),
        u64::from(value.dms_exclusive_acquires),
        u64::from(value.dms_truncates),
        u64::from(value.dms_exclusive_releases),
        u64::from(value.dms_shared_acquires),
        u64::from(value.dms_ready),
        u64::from(value.file_size_checks),
        value.file_size_before,
        value.logical_end,
        u64::from(value.file_grows),
        u64::from(value.mapping_creates),
        u64::from(value.view_maps),
        u64::from(value.records),
        u64::from(value.not_present),
        u64::from(value.mapped),
        u64::from(value.mapped_new),
        u64::from(value.mapped_reuses),
        u64::from(value.selected_pointer.is_some()),
        u64::from(relation.selection_equal),
        value.selected_length as u64,
        value.selected_region.map(u64::from).unwrap_or(0),
        value.selected_runtime_generation.unwrap_or(0),
        u64::from(value.managed_successes),
        u64::from(value.finished),
    ]
}

fn expected_map_receipt_values(
    path: MapRunnerLifecyclePathV1,
    runtime_generation: u64,
    shm_connection_id: u64,
) -> [u64; 33] {
    let created = u64::from(path.is_empty());
    let mapped = u64::from(path.is_mapped());
    let mapped_new = u64::from(path.is_new_mapping());
    [
        runtime_generation,
        shm_connection_id,
        u64::from(path.region()),
        u64::from(REGION_SIZE),
        mode_tag(path),
        managed_path_tag(path.managed_path()),
        managed_dms_path_tag(path.managed_dms_path()),
        1,
        created,
        1 - created,
        created,
        created,
        created,
        created,
        1,
        1,
        file_size_before(path),
        logical_end(path),
        mapped_new,
        mapped_new,
        mapped_new,
        mapped_new,
        1 - mapped,
        mapped,
        mapped_new,
        u64::from(path.is_reuse()),
        mapped,
        mapped,
        if path.is_mapped() {
            u64::from(REGION_SIZE)
        } else {
            0
        },
        if path.is_mapped() {
            u64::from(path.region())
        } else {
            0
        },
        if path.is_mapped() {
            runtime_generation
        } else {
            0
        },
        1,
        1,
    ]
}

fn expected_snapshot_values(path: MapRunnerLifecyclePathV1, after: bool) -> [u64; 14] {
    let (node, regions, dms, file) = if path.is_empty() && !after {
        (0, 0, dms_tag(ManagedSqliteShmTestDmsCustody::Absent), 0)
    } else {
        let regions = match (path, after) {
            (MapRunnerLifecyclePathV1::EmptyObserveNotPresent, true) => 0,
            (MapRunnerLifecyclePathV1::MissingExtendMapped, true) => 2,
            _ => 1,
        };
        (
            1,
            regions,
            dms_tag(ManagedSqliteShmTestDmsCustody::Shared),
            1,
        )
    };
    [1, 0, 0, 1, node, regions, regions, dms, file, 0, 0, 0, 0, 0]
}

fn exact_selector(path: MapRunnerLifecyclePathV1) -> &'static str {
    selector(path_tag(path)).expect("validated Map lifecycle selector")
}

const fn managed_mode_tag(mode: ManagedSqliteShmMapMode) -> u64 {
    match mode {
        ManagedSqliteShmMapMode::Observe => 0,
        ManagedSqliteShmMapMode::Extend => 1,
    }
}

const fn managed_path_tag(path: ManagedSqliteShmTestMapPath) -> u64 {
    match path {
        ManagedSqliteShmTestMapPath::NotPresent => 1,
        ManagedSqliteShmTestMapPath::MappedNew => 2,
        ManagedSqliteShmTestMapPath::MappedReuse => 3,
    }
}

const fn managed_dms_path_tag(path: ManagedSqliteShmTestMapDmsPath) -> u64 {
    match path {
        ManagedSqliteShmTestMapDmsPath::CreatedFirstShared => 1,
        ManagedSqliteShmTestMapDmsPath::NodeLive => 2,
    }
}

fn digest_native_receipt(values: &[u64]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-map-single-region-lifecycle-native-receipt-v3\0");
    // Bind the target identity, installed ABI, topology transition, and lower one-shot ledger.
    // Same-process pointers are reduced to validated presence/equality scalars before this point.
    for value in &values[22..NATIVE_RECEIPT_END] {
        hasher.update(value.to_le_bytes());
    }
    hasher.finalize().into()
}

fn parse_canonical_u64(value: &str) -> anyhow::Result<u64> {
    let parsed = value.parse::<u64>()?;
    if parsed.to_string() != value {
        return Err(anyhow!("Map lifecycle payload scalar is not canonical"));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGISTRATION_ID: u64 = 7;
    const ROUTE_ORDINAL: u64 = 1;
    const RUNTIME_GENERATION: u64 = 9;
    const SHM_CONNECTION_ID: u64 = 11;

    fn binding(path: MapRunnerLifecyclePathV1) -> MapRunnerLifecycleBindingV1 {
        MapRunnerLifecycleBindingV1 {
            path,
            normalized_descriptor_sha256: [0x11; 32],
            case_key_sha256: [0x22; 32],
            full_record_sha256: [0x33; 32],
            plan_sha256: [0x44; 32],
            implementation_sha256: [0x55; 32],
        }
    }

    fn canonical_values(binding: MapRunnerLifecycleBindingV1) -> Vec<u64> {
        let mut values = binding_values(binding);
        values.extend([
            REGISTRATION_ID,
            ROUTE_ORDINAL,
            RUNTIME_GENERATION,
            SHM_CONNECTION_ID,
            u64::from(binding.path.region()),
            u64::from(REGION_SIZE),
            mode_tag(binding.path),
        ]);
        values.extend([
            u64::from(binding.path.region()),
            u64::from(REGION_SIZE),
            binding.path.raw_extend() as u64,
            ffi::SQLITE_OK as u64,
            u64::from(!binding.path.is_mapped()),
            u64::from(binding.path.is_mapped()),
            1,
            1,
            1,
            1,
        ]);
        values.extend(expected_snapshot_values(binding.path, false));
        values.extend(expected_snapshot_values(binding.path, true));
        values.extend(expected_map_receipt_values(
            binding.path,
            RUNTIME_GENERATION,
            SHM_CONNECTION_ID,
        ));
        values.extend([1, 1, 1, 1]);
        values.extend([1, 1, 3]);
        values.extend([1, 1, 1, 1]);
        assert_eq!(values.len(), REPORT_VALUE_COUNT);
        values
    }

    fn payload(binding: MapRunnerLifecycleBindingV1, values: &[u64]) -> String {
        format!(
            "{REPORT_VERSION},{},{}",
            exact_selector(binding.path),
            values
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    fn assert_rejected(binding: MapRunnerLifecycleBindingV1, values: &[u64]) {
        assert!(validate_payload(&payload(binding, values), binding).is_err());
    }

    #[test]
    fn accepts_canonical_payload_for_all_six_paths() {
        for path in [
            MapRunnerLifecyclePathV1::EmptyObserveNotPresent,
            MapRunnerLifecyclePathV1::EmptyExtendMapped,
            MapRunnerLifecyclePathV1::ReuseObserveMapped,
            MapRunnerLifecyclePathV1::ReuseExtendMapped,
            MapRunnerLifecyclePathV1::MissingObserveNotPresent,
            MapRunnerLifecyclePathV1::MissingExtendMapped,
        ] {
            let binding = binding(path);
            let values = canonical_values(binding);
            assert!(validate_payload(&payload(binding, &values), binding).is_ok());
        }
    }

    #[test]
    fn rejects_tamper_in_every_bound_section() {
        let binding = binding(MapRunnerLifecyclePathV1::MissingExtendMapped);
        for index in [0, 22, 29, 39, 53, 67, 93, 100, 104, 107] {
            let mut values = canonical_values(binding);
            values[index] ^= 1;
            assert_rejected(binding, &values);
        }
    }

    #[test]
    fn rejects_zero_or_divergent_target_identity() {
        let binding = binding(MapRunnerLifecyclePathV1::ReuseObserveMapped);
        for index in 22..=25 {
            let mut values = canonical_values(binding);
            values[index] = 0;
            assert_rejected(binding, &values);
        }
        for (receipt_index, identity_index) in [(67, 24), (68, 25)] {
            let mut values = canonical_values(binding);
            values[receipt_index] = values[identity_index] + 1;
            assert_rejected(binding, &values);
        }
    }

    #[test]
    fn rejects_noncanonical_numeric_and_wrong_width_or_header() {
        let binding = binding(MapRunnerLifecyclePathV1::EmptyObserveNotPresent);
        let values = canonical_values(binding);
        let canonical = payload(binding, &values);
        let mut fields = canonical.split(',').map(str::to_owned).collect::<Vec<_>>();
        fields[2] = "01".to_string();
        assert!(validate_payload(&fields.join(","), binding).is_err());
        fields = canonical.split(',').map(str::to_owned).collect();
        fields.pop();
        assert!(validate_payload(&fields.join(","), binding).is_err());
        assert!(validate_payload(&canonical.replacen(REPORT_VERSION, "a2mapq2", 1), binding)
            .is_err());
    }
}
