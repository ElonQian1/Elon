use super::{
    AuthorizedInstalledWorkAdmission, ComputePluginWorkAdmissionRecoveryKey,
    DurableWorkAdmittedPluginSlot, InstalledWorkAdmissionOutcomeUncertainCustody,
    InstalledWorkAdmissionRecoveryStoreFailure, ValidatedInstalledWorkAdmissionStorePermit,
};

/// Persists one immutable source/receipt and advances the exact current head in one Store
/// transaction. No runtime, Ready capability, attempt, or inventory scalar is accepted here.
pub(in crate::node_agent_compute_plugin_host) fn persist_authorized_work_admission<'root>(
    authorized: AuthorizedInstalledWorkAdmission<'root, '_>,
) -> Result<DurableWorkAdmittedPluginSlot<'root>, InstalledWorkAdmissionRecoveryStoreFailure<'root>>
{
    let recovery_key = ComputePluginWorkAdmissionRecoveryKey::from_authorized(&authorized);
    let store_result = {
        let permit = ValidatedInstalledWorkAdmissionStorePermit::new(&authorized);
        authorized
            .authority_session()
            .persist_installed_work_admission(permit)
    };
    if let Err(error) = store_result {
        let (revalidated, _) = authorized.into_parts();
        return Err(InstalledWorkAdmissionRecoveryStoreFailure::new(
            error,
            InstalledWorkAdmissionOutcomeUncertainCustody::new(revalidated, recovery_key),
        ));
    }
    let (revalidated, receipts) = authorized.into_parts();
    Ok(DurableWorkAdmittedPluginSlot::new(revalidated, receipts))
}
