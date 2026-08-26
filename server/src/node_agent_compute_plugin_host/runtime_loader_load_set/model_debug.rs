use std::fmt;

use super::model::*;

impl fmt::Debug for LoaderTransitionAuthorityCustody<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LoaderTransitionAuthorityCustody")
            .field("work_admission_receipts", &self.work_admission_receipts)
            .field("promotion_receipts", &self.promotion_receipts)
            .field("extraction_plan", &"<retained>")
            .field("verified_artifacts", &"<retained-share-none>")
            .field("staging_root", &"<retained-root-borrow>")
            .field("staging_root_lock", &"<owned-exact-root-lock-lease>")
            .field("staging_relative_root", &"<redacted>")
            .field("staging_seal", &"<retained-share-none>")
            .finish()
    }
}

impl fmt::Debug for SealedComputePluginRunnerImage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SealedComputePluginRunnerImage")
            .field("package_file_count", &self.package_files.len())
            .field("runner_ordinal", &self.runner_ordinal)
            .field(
                "working_directory_location",
                &"<package-root-or-plan-ordinal>",
            )
            .field(
                "namespace_directory_count",
                &self.namespace_directories.len(),
            )
            .field("application_path", &"<derived-from-retained-runner>")
            .field(
                "startup_import_resolution_profile_digest",
                &"<derived-from-prerequisite>",
            )
            .field(
                "startup_import_namespace_authority_digest",
                &"<derived-from-prerequisite>",
            )
            .finish()
    }
}

impl fmt::Debug for LoaderLockedWorkAdmittedPluginSlot<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LoaderLockedWorkAdmittedPluginSlot")
            .field("authority", &self.authority)
            .field("image", &self.image)
            .finish()
    }
}
