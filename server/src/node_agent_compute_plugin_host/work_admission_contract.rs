//! Linear work-admission contract for one durable installed plugin slot.
//!
//! This boundary seals launch inputs and numeric ceilings. It deliberately cannot start a
//! runtime, mint a Ready capability, create an attempt, or infer missing enforcement limits.
//! The only initial ordering is: guarded installed rehash, later trusted time, apply the signed
//! `reauthorize_existing` plan, then mint a fresh post-revalidation Store session. Applying that
//! successor plan before the guarded rehash must fail closed because it closes the candidate source.

mod adoption;
mod authorization;
mod capability;
mod profile;
mod receipt;
mod recovery;
mod revalidation;
mod store;

pub(in crate::node_agent_compute_plugin_host) use adoption::{
    adopt_recovered_work_admission, begin_work_admission_recovery_revalidation,
};
pub(in crate::node_agent_compute_plugin_host) use authorization::{
    authorize_installed_work_admission, AuthorizedInstalledWorkAdmission,
    InstalledWorkAdmissionAuthorizationFailure, ValidatedInstalledWorkAdmissionStorePermit,
};
pub(in crate::node_agent_compute_plugin_host) use capability::{
    DurableWorkAdmittedPluginSlot, InstalledWorkAdmissionRevalidationCustody,
    PendingInstalledWorkAdmissionRevalidation, RevalidatedInstalledWorkAdmission,
};
pub(in crate::node_agent_compute_plugin_host) use profile::ComputePluginWorkAdmissionLaunchProfile;
pub(in crate::node_agent_compute_plugin_host) use receipt::{
    ComputePluginWorkAdmissionAuthorityTransition, ComputePluginWorkAdmissionGenerationTransition,
    ComputePluginWorkAdmissionPlanBinding, ComputePluginWorkAdmissionQuiescence,
    ComputePluginWorkAdmissionReceipt, ComputePluginWorkAdmissionReceiptPair,
    ComputePluginWorkAdmissionSource, HashedComputePluginWorkAdmissionReceipt,
    HashedComputePluginWorkAdmissionSource,
};
pub(in crate::node_agent_compute_plugin_host) use recovery::{
    ComputePluginWorkAdmissionExpectation, ComputePluginWorkAdmissionRecoveryKey,
    ComputePluginWorkAdmissionRecoveryOutcome, InstalledWorkAdmissionOutcomeUncertainCustody,
    InstalledWorkAdmissionRecoveryAdoption, InstalledWorkAdmissionRecoveryAdoptionFailure,
    InstalledWorkAdmissionRecoveryAdoptionPhase, InstalledWorkAdmissionRecoveryRevalidationFailure,
    InstalledWorkAdmissionRecoveryStoreFailure, InstalledWorkAdmissionStorePhase,
    PendingInstalledWorkAdmissionRecoveryAdoption,
};
pub(in crate::node_agent_compute_plugin_host) use revalidation::{
    begin_installed_work_admission_revalidation, complete_installed_work_admission_revalidation,
    InstalledWorkAdmissionRevalidationFailure, InstalledWorkAdmissionRevalidationPhase,
};
pub(in crate::node_agent_compute_plugin_host) use store::persist_authorized_work_admission;

pub(super) const SOURCE_SCHEMA: &str = "elon.compute_plugin.work_admission_source.v1";
pub(super) const HASHED_SOURCE_SCHEMA: &str = "elon.compute_plugin.hashed_work_admission_source.v1";
pub(super) const RECEIPT_SCHEMA: &str = "elon.compute_plugin.work_admission_receipt.v1";
pub(super) const HASHED_RECEIPT_SCHEMA: &str =
    "elon.compute_plugin.hashed_work_admission_receipt.v1";
pub(super) const CANONICALIZATION: &str = "RFC8785-JCS";
pub(super) const DIGEST_ALGORITHM: &str = "sha256";
