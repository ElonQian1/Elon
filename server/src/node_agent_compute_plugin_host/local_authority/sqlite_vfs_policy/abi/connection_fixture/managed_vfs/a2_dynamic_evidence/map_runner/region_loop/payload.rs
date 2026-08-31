//! Canonical q4 encoding and parent validation for successful Map region loops.

use anyhow::anyhow;
use rusqlite::ffi;
use sha2::{Digest, Sha256};

use crate::node_agent_managed_fs::{
    ManagedSqliteShmMapMode, ManagedSqliteShmTestDmsCustody, ManagedSqliteShmTestMapDmsPath,
    ManagedSqliteShmTestMapPath, ManagedSqliteShmTestMapReceipt,
    ManagedSqliteShmTestTargetSnapshot,
};

use super::super::super::super::connection::ManagedTestShmMapCallbackObservation;
use super::super::super::child::map_region_loop::{REPORT_VALUE_COUNT, REPORT_VERSION};
use super::fixture::{
    dms_tag, file_size_before, logical_end, snapshot_values, ValidatedPointerRelationV1,
};
#[cfg(test)]
use super::MapRunnerRegionLoopFamilyV1;
use super::{exact_selector, MapRunnerRegionLoopBindingV1, REGION_SIZE};

const BINDING_END: usize = 24;
const NATIVE_RECEIPT_END: usize = 103;

pub(in super::super) struct ValidatedRegionLoopPayloadV1 {
    pub(in super::super) registration_id: u64,
    pub(in super::super) native_receipt_sha256: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub(super) fn encode(
    binding: MapRunnerRegionLoopBindingV1,
    registration_id: u64,
    route_ordinal: u64,
    runtime_generation: u64,
    shm_connection_id: u64,
    callback: ManagedTestShmMapCallbackObservation,
    before: ManagedSqliteShmTestTargetSnapshot,
    after: ManagedSqliteShmTestTargetSnapshot,
    receipt: ManagedSqliteShmTestMapReceipt,
    relation: ValidatedPointerRelationV1,
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
        u64::from(binding.target_region),
        u64::from(REGION_SIZE),
        1,
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
        exact_selector(binding).expect("validated Map region-loop selector"),
        values
            .into_iter()
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(",")
    )
}

pub(in super::super) fn validate_payload(
    payload: &str,
    binding: MapRunnerRegionLoopBindingV1,
) -> anyhow::Result<ValidatedRegionLoopPayloadV1> {
    super::validate_binding(binding)?;
    let selector = exact_selector(binding)?;
    let mut fields = payload.split(',');
    if fields.next() != Some(REPORT_VERSION) || fields.next() != Some(selector.as_str()) {
        return Err(anyhow!("Map region-loop payload identity mismatch"));
    }
    let values = fields
        .map(parse_canonical_u64)
        .collect::<anyhow::Result<Vec<_>>>()?;
    if values.len() != REPORT_VALUE_COUNT || values[..BINDING_END] != binding_values(binding) {
        return Err(anyhow!("Map region-loop payload program binding mismatch"));
    }
    if values[24..28].contains(&0)
        || values[24..31]
            != [
                values[24],
                values[25],
                values[26],
                values[27],
                u64::from(binding.target_region),
                u64::from(REGION_SIZE),
                1,
            ]
        || values[31..41]
            != [
                u64::from(binding.target_region),
                u64::from(REGION_SIZE),
                1,
                ffi::SQLITE_OK as u64,
                0,
                1,
                1,
                1,
                1,
                1,
            ]
    {
        return Err(anyhow!(
            "Map region-loop payload installed-ABI binding mismatch"
        ));
    }
    if values[41..55] != expected_snapshot_values(binding, false)
        || values[55..69] != expected_snapshot_values(binding, true)
        || values[69..103] != expected_map_receipt_values(binding, values[26], values[27])
        || values[103..107] != [1, 1, 1, 1]
        || values[107..110] != [1, 1, 3]
        || values[110..114] != [1, 1, 1, 1]
    {
        return Err(anyhow!("Map region-loop payload native receipt mismatch"));
    }
    Ok(ValidatedRegionLoopPayloadV1 {
        registration_id: values[24],
        native_receipt_sha256: digest_native_receipt(&values),
    })
}

fn binding_values(binding: MapRunnerRegionLoopBindingV1) -> Vec<u64> {
    let mut values = vec![
        binding.family.tag(),
        u64::from(binding.target_region),
        u64::from(binding.regions_to_create),
        1,
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

fn map_receipt_values(
    value: ManagedSqliteShmTestMapReceipt,
    relation: ValidatedPointerRelationV1,
) -> [u64; 34] {
    [
        value.runtime_generation,
        value.shm_connection_id,
        u64::from(value.expectation.region),
        u64::from(value.expectation.region_size),
        u64::from(value.expectation.regions_to_create),
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
    binding: MapRunnerRegionLoopBindingV1,
    runtime_generation: u64,
    shm_connection_id: u64,
) -> [u64; 34] {
    let created = u64::from(binding.family.is_empty());
    let count = u64::from(binding.regions_to_create);
    [
        runtime_generation,
        shm_connection_id,
        u64::from(binding.target_region),
        u64::from(REGION_SIZE),
        count,
        1,
        managed_path_tag(ManagedSqliteShmTestMapPath::MappedNew),
        managed_dms_path_tag(binding.family.managed_dms_path()),
        1,
        created,
        1 - created,
        created,
        created,
        created,
        created,
        1,
        1,
        file_size_before(binding),
        logical_end(binding),
        1,
        count,
        count,
        count,
        0,
        1,
        1,
        0,
        1,
        1,
        u64::from(REGION_SIZE),
        u64::from(binding.target_region),
        runtime_generation,
        1,
        1,
    ]
}

fn expected_snapshot_values(binding: MapRunnerRegionLoopBindingV1, after: bool) -> [u64; 14] {
    let (node, regions, dms, file) = if binding.family.is_empty() && !after {
        (0, 0, dms_tag(ManagedSqliteShmTestDmsCustody::Absent), 0)
    } else {
        (
            1,
            if after {
                u64::from(binding.target_region) + 1
            } else {
                1
            },
            dms_tag(ManagedSqliteShmTestDmsCustody::Shared),
            1,
        )
    };
    [1, 0, 0, 1, node, regions, regions, dms, file, 0, 0, 0, 0, 0]
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
    hasher.update(b"elon-map-region-loop-success-native-receipt-v4\0");
    for value in &values[BINDING_END..NATIVE_RECEIPT_END] {
        hasher.update(value.to_le_bytes());
    }
    hasher.finalize().into()
}

fn parse_canonical_u64(value: &str) -> anyhow::Result<u64> {
    let parsed = value.parse::<u64>()?;
    if parsed.to_string() != value {
        return Err(anyhow!("Map region-loop payload scalar is not canonical"));
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

    fn binding(
        family: MapRunnerRegionLoopFamilyV1,
        target_region: u32,
        regions_to_create: u16,
    ) -> MapRunnerRegionLoopBindingV1 {
        MapRunnerRegionLoopBindingV1 {
            family,
            target_region,
            regions_to_create,
            normalized_descriptor_sha256: [0x11; 32],
            case_key_sha256: [0x22; 32],
            full_record_sha256: [0x33; 32],
            plan_sha256: [0x44; 32],
            implementation_sha256: [0x55; 32],
        }
    }

    fn canonical_values(binding: MapRunnerRegionLoopBindingV1) -> Vec<u64> {
        let mut values = binding_values(binding);
        values.extend([
            REGISTRATION_ID,
            ROUTE_ORDINAL,
            RUNTIME_GENERATION,
            SHM_CONNECTION_ID,
            u64::from(binding.target_region),
            u64::from(REGION_SIZE),
            1,
        ]);
        values.extend([
            u64::from(binding.target_region),
            u64::from(REGION_SIZE),
            1,
            ffi::SQLITE_OK as u64,
            0,
            1,
            1,
            1,
            1,
            1,
        ]);
        values.extend(expected_snapshot_values(binding, false));
        values.extend(expected_snapshot_values(binding, true));
        values.extend(expected_map_receipt_values(
            binding,
            RUNTIME_GENERATION,
            SHM_CONNECTION_ID,
        ));
        values.extend([1, 1, 1, 1]);
        values.extend([1, 1, 3]);
        values.extend([1, 1, 1, 1]);
        assert_eq!(values.len(), REPORT_VALUE_COUNT);
        values
    }

    fn payload(binding: MapRunnerRegionLoopBindingV1, values: &[u64]) -> String {
        format!(
            "{REPORT_VERSION},{},{}",
            exact_selector(binding).unwrap(),
            values
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    #[test]
    fn accepts_canonical_boundary_members_for_both_families() {
        for binding in [
            binding(
                MapRunnerRegionLoopFamilyV1::CreatedFirstEmptyExtendMapped,
                0,
                1,
            ),
            binding(
                MapRunnerRegionLoopFamilyV1::CreatedFirstEmptyExtendMapped,
                255,
                256,
            ),
            binding(
                MapRunnerRegionLoopFamilyV1::NodeLiveMissingExtendMapped,
                1,
                1,
            ),
            binding(
                MapRunnerRegionLoopFamilyV1::NodeLiveMissingExtendMapped,
                255,
                255,
            ),
        ] {
            let values = canonical_values(binding);
            assert!(validate_payload(&payload(binding, &values), binding).is_ok());
        }
    }

    #[test]
    fn rejects_tamper_in_axes_identity_topology_and_order_counts() {
        let binding = binding(
            MapRunnerRegionLoopFamilyV1::CreatedFirstEmptyExtendMapped,
            2,
            3,
        );
        for index in [0, 1, 2, 24, 28, 31, 41, 55, 69, 73, 89, 103, 107, 110] {
            let mut values = canonical_values(binding);
            values[index] ^= 1;
            assert!(validate_payload(&payload(binding, &values), binding).is_err());
        }
    }

    #[test]
    fn rejects_divergent_binding_and_noncanonical_or_wrong_width_payloads() {
        let binding = binding(
            MapRunnerRegionLoopFamilyV1::NodeLiveMissingExtendMapped,
            2,
            2,
        );
        let values = canonical_values(binding);
        let divergent = binding(
            MapRunnerRegionLoopFamilyV1::NodeLiveMissingExtendMapped,
            3,
            3,
        );
        assert!(validate_payload(&payload(binding, &values), divergent).is_err());
        let canonical = payload(binding, &values);
        let mut fields = canonical.split(',').map(str::to_owned).collect::<Vec<_>>();
        fields[2] = "01".to_string();
        assert!(validate_payload(&fields.join(","), binding).is_err());
        fields = canonical.split(',').map(str::to_owned).collect();
        fields.pop();
        assert!(validate_payload(&fields.join(","), binding).is_err());
    }
}
