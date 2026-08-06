use anyhow::anyhow;

use super::ReconciledComputePluginPartFile;
use crate::node_agent_managed_fs::ManagedFileSegmentWritePhase;

#[path = "write/types.rs"]
mod types;

pub(in crate::node_agent_compute_plugin_host) use types::{
    ComputePluginSegmentWriteFailure, ComputePluginSegmentWritePhase, SyncedComputePluginPartFile,
};

/// Consumes one reconciled claim/file capability and writes exactly the authorized segment. The
/// cancellation guard was captured before Store claim creation and is checked before each bounded
/// file write. Future network code must additionally check it before every socket read.
pub(in crate::node_agent_compute_plugin_host) fn write_compute_plugin_part_segment(
    mut reconciled: ReconciledComputePluginPartFile,
    bytes: &[u8],
) -> std::result::Result<SyncedComputePluginPartFile, ComputePluginSegmentWriteFailure> {
    let expected_len = match usize::try_from(reconciled.authorized.length_bytes()) {
        Ok(expected_len) if expected_len == bytes.len() && expected_len > 0 => expected_len,
        _ => {
            return Err(ComputePluginSegmentWriteFailure::from_reconciled(
                ComputePluginSegmentWritePhase::Payload,
                anyhow!("COMPUTE_PLUGIN_SEGMENT_PAYLOAD_LENGTH_MISMATCH"),
                reconciled,
                false,
            ));
        }
    };
    debug_assert_eq!(expected_len, bytes.len());
    let expected_offset = match u64::try_from(reconciled.authorized.offset_bytes()) {
        Ok(expected_offset) => expected_offset,
        Err(error) => {
            return Err(ComputePluginSegmentWriteFailure::from_reconciled(
                ComputePluginSegmentWritePhase::Payload,
                error,
                reconciled,
                false,
            ));
        }
    };

    let write_result = {
        let authorized = &reconciled.authorized;
        reconciled
            .file
            .write_segment_sync_and_revalidate(expected_offset, bytes, || {
                authorized.ensure_not_canceled()
            })
    };
    match write_result {
        Ok(sync_completed_at) => {
            let ReconciledComputePluginPartFile {
                authorized,
                file,
                root_lock_lease,
                ..
            } = reconciled;
            Ok(SyncedComputePluginPartFile {
                authorized,
                file,
                root_lock_lease,
                sync_completed_at,
            })
        }
        Err(failure) => {
            let phase = map_phase(failure.phase());
            let mutation_was_attempted = failure.filesystem_mutation_was_attempted();
            Err(ComputePluginSegmentWriteFailure::from_reconciled(
                phase,
                failure.into_error(),
                reconciled,
                mutation_was_attempted,
            ))
        }
    }
}

fn map_phase(phase: ManagedFileSegmentWritePhase) -> ComputePluginSegmentWritePhase {
    match phase {
        ManagedFileSegmentWritePhase::PrewriteRevalidate => {
            ComputePluginSegmentWritePhase::PrewriteRevalidate
        }
        ManagedFileSegmentWritePhase::Seek => ComputePluginSegmentWritePhase::Seek,
        ManagedFileSegmentWritePhase::Cancellation => ComputePluginSegmentWritePhase::Canceled,
        ManagedFileSegmentWritePhase::Write => ComputePluginSegmentWritePhase::Write,
        ManagedFileSegmentWritePhase::Flush => ComputePluginSegmentWritePhase::Flush,
        ManagedFileSegmentWritePhase::Sync => ComputePluginSegmentWritePhase::Sync,
        ManagedFileSegmentWritePhase::PostSyncRevalidate => {
            ComputePluginSegmentWritePhase::PostSyncRevalidate
        }
    }
}
