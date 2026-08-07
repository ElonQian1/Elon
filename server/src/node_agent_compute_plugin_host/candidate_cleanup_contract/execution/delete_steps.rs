use std::{cmp::Ordering, collections::VecDeque, path::Path};

use anyhow::{bail, Error, Result};

use super::ComputePluginCandidateCleanupStepEvidence;
use crate::node_agent_compute_plugin_host::candidate_cleanup_contract::topology::builder::CandidateCleanupTopologyObjectInput;
use crate::node_agent_compute_plugin_host::candidate_cleanup_contract::HashedCandidateCleanupExpectedObject;
use crate::node_agent_managed_fs::{
    ManagedDeleteDisposition, ManagedObjectBinding, PinnedManagedDirectory, PinnedManagedFile,
};

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

pub(super) enum PendingCleanupObject {
    File(PendingCleanupFile),
    Directory(PendingCleanupDirectory),
}

impl PendingCleanupObject {
    pub(super) fn validate_expected(
        &self,
        expected: &HashedCandidateCleanupExpectedObject,
    ) -> Result<()> {
        let input = match self {
            Self::File(file) => file.topology_input()?,
            Self::Directory(directory) => directory.topology_input()?,
        };
        let object = expected.object();
        let expected_size_bytes = input
            .expected_size_bytes
            .map(i64::try_from)
            .transpose()
            .map_err(|_| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_SIZE_OVERFLOW"))?;
        if input.logical_kind != object.logical_kind()
            || input.relative_path != object.relative_path()
            || input.expected_identity_digest != object.expected_identity_digest()
            || input.expected_parent_identity_digest != object.expected_parent_identity_digest()
            || input.expected_content_digest.as_deref() != object.expected_content_digest()
            || expected_size_bytes != object.expected_size_bytes()
        {
            bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_PENDING_OBJECT_CHANGED");
        }
        Ok(())
    }

    pub(super) fn set_delete_disposition_exact(
        self,
    ) -> std::result::Result<ManagedDeleteDisposition, (Error, Self)> {
        match self {
            Self::File(pending) => set_file_delete_disposition(pending),
            Self::Directory(pending) => set_directory_delete_disposition(pending),
        }
    }
}

fn set_file_delete_disposition(
    pending: PendingCleanupFile,
) -> std::result::Result<ManagedDeleteDisposition, (Error, PendingCleanupObject)> {
    let PendingCleanupFile {
        object_kind,
        logical_path,
        content_digest,
        expected_identity_digest,
        file,
    } = pending;
    match file.set_delete_disposition_exact() {
        Ok(disposition) => Ok(disposition),
        Err(failure) => {
            let (error, file) = failure.into_parts();
            Err((
                Error::new(error),
                PendingCleanupObject::File(PendingCleanupFile {
                    object_kind,
                    logical_path,
                    content_digest,
                    expected_identity_digest,
                    file,
                }),
            ))
        }
    }
}

fn set_directory_delete_disposition(
    pending: PendingCleanupDirectory,
) -> std::result::Result<ManagedDeleteDisposition, (Error, PendingCleanupObject)> {
    let PendingCleanupDirectory {
        object_kind,
        logical_path,
        directory,
    } = pending;
    match directory.set_delete_disposition_exact() {
        Ok(disposition) => Ok(disposition),
        Err(failure) => {
            let (error, directory) = failure.into_parts();
            Err((
                Error::new(error),
                PendingCleanupObject::Directory(PendingCleanupDirectory {
                    object_kind,
                    logical_path,
                    directory,
                }),
            ))
        }
    }
}

impl PendingCleanupFile {
    pub(super) fn topology_input(&self) -> Result<CandidateCleanupTopologyObjectInput> {
        let binding = self.file.object_binding();
        validate_topology_binding(
            &self.logical_path,
            binding,
            false,
            Some(&self.expected_identity_digest),
        )?;
        Ok(CandidateCleanupTopologyObjectInput {
            logical_kind: self.object_kind,
            relative_path: self.logical_path.clone(),
            expected_identity_digest: binding.identity_digest().to_string(),
            expected_parent_identity_digest: binding.parent_identity_digest().to_string(),
            expected_content_digest: Some(self.content_digest.clone()),
            expected_size_bytes: Some(self.file.len_bytes()),
        })
    }
}

impl PendingCleanupDirectory {
    pub(super) fn topology_input(&self) -> Result<CandidateCleanupTopologyObjectInput> {
        let binding = self.directory.object_binding().ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_DIRECTORY_BINDING_MISSING")
        })?;
        validate_topology_binding(&self.logical_path, binding, true, None)?;
        Ok(CandidateCleanupTopologyObjectInput {
            logical_kind: self.object_kind,
            relative_path: self.logical_path.clone(),
            expected_identity_digest: binding.identity_digest().to_string(),
            expected_parent_identity_digest: binding.parent_identity_digest().to_string(),
            expected_content_digest: None,
            expected_size_bytes: None,
        })
    }
}

fn validate_topology_binding(
    logical_path: &str,
    binding: &ManagedObjectBinding,
    expect_directory: bool,
    expected_identity_digest: Option<&str>,
) -> Result<()> {
    let relative_name = Path::new(logical_path)
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_OBJECT_NAME_MISSING"))?;
    if binding.is_directory() != expect_directory
        || binding.relative_name() != relative_name
        || expected_identity_digest.is_some_and(|expected| expected != binding.identity_digest())
    {
        bail!("COMPUTE_PLUGIN_CANDIDATE_CLEANUP_OBJECT_BINDING_CHANGED");
    }
    Ok(())
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
