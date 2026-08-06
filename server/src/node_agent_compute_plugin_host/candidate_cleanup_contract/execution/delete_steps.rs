use std::{cmp::Ordering, collections::VecDeque};

use anyhow::{Error, Result};

use super::ComputePluginCandidateCleanupStepEvidence;
use crate::node_agent_managed_fs::{PinnedManagedDirectory, PinnedManagedFile};

pub(super) struct PendingCleanupFile {
    pub object_kind: &'static str,
    pub logical_path: String,
    pub content_digest: String,
    pub expected_identity_digest: String,
    pub file: PinnedManagedFile,
}

pub(super) struct PendingCleanupDirectory {
    pub object_kind: &'static str,
    pub logical_path: String,
    pub directory: PinnedManagedDirectory,
}

pub(super) fn ordered_files(mut files: Vec<PendingCleanupFile>) -> VecDeque<PendingCleanupFile> {
    files.sort_by(|left, right| left.logical_path.cmp(&right.logical_path));
    files.into()
}

pub(super) fn ordered_staging_directories(
    mut directories: Vec<PendingCleanupDirectory>,
) -> VecDeque<PendingCleanupDirectory> {
    directories
        .sort_by(|left, right| staging_directory_order(&left.logical_path, &right.logical_path));
    directories.into()
}

pub(super) fn delete_file(
    completed: &mut Vec<ComputePluginCandidateCleanupStepEvidence>,
    pending: PendingCleanupFile,
) -> std::result::Result<(), (Error, PendingCleanupFile)> {
    let PendingCleanupFile {
        object_kind,
        logical_path,
        content_digest,
        expected_identity_digest,
        file,
    } = pending;
    match file.delete_exact() {
        Ok(evidence) => {
            let actual_identity_digest = evidence.identity_digest().map(str::to_string);
            debug_assert_eq!(
                actual_identity_digest.as_deref(),
                Some(expected_identity_digest.as_str())
            );
            completed.push(ComputePluginCandidateCleanupStepEvidence {
                sequence: next_sequence(completed),
                object_kind: object_kind.to_string(),
                logical_path,
                content_digest: Some(content_digest),
                file_identity_digest: actual_identity_digest.clone(),
            });
            Ok(())
        }
        Err(failure) => {
            let (error, file) = failure.into_parts();
            Err((
                Error::new(error),
                PendingCleanupFile {
                    object_kind,
                    logical_path,
                    content_digest,
                    expected_identity_digest,
                    file,
                },
            ))
        }
    }
}

pub(super) fn delete_directory(
    completed: &mut Vec<ComputePluginCandidateCleanupStepEvidence>,
    pending: PendingCleanupDirectory,
) -> std::result::Result<(), (Error, PendingCleanupDirectory)> {
    let PendingCleanupDirectory {
        object_kind,
        logical_path,
        directory,
    } = pending;
    match directory.delete_exact() {
        Ok(evidence) => {
            debug_assert!(evidence.is_directory());
            completed.push(ComputePluginCandidateCleanupStepEvidence {
                sequence: next_sequence(completed),
                object_kind: object_kind.to_string(),
                logical_path,
                content_digest: None,
                file_identity_digest: None,
            });
            Ok(())
        }
        Err(failure) => {
            let (error, directory) = failure.into_parts();
            Err((
                Error::new(error),
                PendingCleanupDirectory {
                    object_kind,
                    logical_path,
                    directory,
                },
            ))
        }
    }
}

pub(super) fn delete_optional_directory(
    completed: &mut Vec<ComputePluginCandidateCleanupStepEvidence>,
    pending: &mut Option<PendingCleanupDirectory>,
) -> Result<()> {
    let directory = pending
        .take()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DIRECTORY_MISSING"))?;
    match delete_directory(completed, directory) {
        Ok(()) => Ok(()),
        Err((error, retained)) => {
            *pending = Some(retained);
            Err(error)
        }
    }
}

fn staging_directory_order(left: &str, right: &str) -> Ordering {
    let left_depth = left.split('/').count();
    let right_depth = right.split('/').count();
    right_depth.cmp(&left_depth).then_with(|| left.cmp(right))
}

fn next_sequence(completed: &[ComputePluginCandidateCleanupStepEvidence]) -> i64 {
    i64::try_from(completed.len() + 1).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::staging_directory_order;

    #[test]
    fn staging_directories_are_ordered_deepest_first_then_lexically() {
        let mut paths = vec!["staging/run/a", "staging/run/a/b", "staging/run/c"];
        paths.sort_by(|left, right| staging_directory_order(left, right));
        assert_eq!(
            paths,
            vec!["staging/run/a/b", "staging/run/a", "staging/run/c"]
        );
    }
}
