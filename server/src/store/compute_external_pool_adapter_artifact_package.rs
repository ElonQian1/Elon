//! Immutable V232 receipts for bounded static Adapter package inspection.

mod read;
mod types;
mod write;

pub(in crate::store) use read::{
    artifact_package_authority_on, current_artifact_package_authority_on,
};

pub(crate) use types::{
    CreateExternalPoolAdapterArtifactPackageReceipt,
    ExternalPoolAdapterArtifactPackageCurrentnessReceipt,
    ExternalPoolAdapterArtifactPackageInspectionTarget, ExternalPoolAdapterArtifactPackageSummary,
    ExternalPoolAdapterArtifactPackageWriteReceipt,
};
