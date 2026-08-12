//! Immutable V232 receipts for bounded static Adapter package inspection.

mod read;
mod types;
mod write;

pub(crate) use types::{
    CreateExternalPoolAdapterArtifactPackageReceipt,
    ExternalPoolAdapterArtifactPackageCurrentnessReceipt,
    ExternalPoolAdapterArtifactPackageInspectionTarget, ExternalPoolAdapterArtifactPackageSummary,
    ExternalPoolAdapterArtifactPackageWriteReceipt,
};
