use std::{error::Error as StdError, fmt, time::Instant};

use anyhow::Error;

use crate::node_agent_compute_plugin_host::local_authority::{
    ComputePluginInstalledWorkAdmissionAuthorityFacts,
    ComputePluginPostRevalidationWorkAdmissionAuthoritySession,
};

use super::{
    profile::ComputePluginWorkAdmissionLaunchProfile, receipt::build_work_admission_receipts,
    ComputePluginWorkAdmissionReceiptPair, RevalidatedInstalledWorkAdmission,
};

#[must_use = "authorized work admission must be stored or returned with custody"]
pub(in crate::node_agent_compute_plugin_host) struct AuthorizedInstalledWorkAdmission<
    'root,
    'authority,
> {
    revalidated: RevalidatedInstalledWorkAdmission<'root>,
    authority_session: ComputePluginPostRevalidationWorkAdmissionAuthoritySession<'authority>,
    facts: ComputePluginInstalledWorkAdmissionAuthorityFacts,
    receipts: ComputePluginWorkAdmissionReceiptPair,
}

pub(in crate::node_agent_compute_plugin_host) struct InstalledWorkAdmissionAuthorizationFailure<
    'root,
> {
    error: Error,
    revalidated: RevalidatedInstalledWorkAdmission<'root>,
}

/// Borrowed linear proof for the exact Store postcondition. It cannot be serialized or retained.
pub(in crate::node_agent_compute_plugin_host) struct ValidatedInstalledWorkAdmissionStorePermit<
    'permit,
    'root,
> {
    authorized: &'permit AuthorizedInstalledWorkAdmission<'root, 'permit>,
}

pub(in crate::node_agent_compute_plugin_host) fn authorize_installed_work_admission<
    'root,
    'authority,
>(
    revalidated: RevalidatedInstalledWorkAdmission<'root>,
    authority_session: ComputePluginPostRevalidationWorkAdmissionAuthoritySession<'authority>,
) -> Result<
    AuthorizedInstalledWorkAdmission<'root, 'authority>,
    InstalledWorkAdmissionAuthorizationFailure<'root>,
> {
    authorize(revalidated, authority_session).map_err(|(error, revalidated)| {
        InstalledWorkAdmissionAuthorizationFailure { error, revalidated }
    })
}

fn authorize<'root, 'authority>(
    revalidated: RevalidatedInstalledWorkAdmission<'root>,
    authority_session: ComputePluginPostRevalidationWorkAdmissionAuthoritySession<'authority>,
) -> std::result::Result<
    AuthorizedInstalledWorkAdmission<'root, 'authority>,
    (Error, RevalidatedInstalledWorkAdmission<'root>),
> {
    if let Err(error) = revalidated.trusted_time().ensure_live(Instant::now()) {
        return Err((error, revalidated));
    }
    let revalidated_now_ms = revalidated.trusted_time().trusted_now().timestamp_millis();
    if !authority_session.was_observed_strictly_after(revalidated.revalidated_at())
        || !authority_session.plan_application_matches_observation(revalidated.trusted_time())
        || authority_session.trusted_now_ms() < revalidated_now_ms
        || authority_session.installation_id_digest()
            != revalidated.trusted_time().installation_id_digest()
        || authority_session.clock_epoch_digest() != revalidated.trusted_time().clock_epoch_digest()
    {
        return Err((
            anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_AUTHORITY_NOT_POST_REVALIDATION"),
            revalidated,
        ));
    }
    let facts = match authority_session.read_installed_work_admission_binding(&revalidated) {
        Ok(value) => value,
        Err(error) => return Err((error, revalidated)),
    };
    if facts.admitted_at_ms() != authority_session.trusted_now_ms() {
        return Err((
            anyhow::anyhow!("COMPUTE_PLUGIN_WORK_ADMISSION_AUTHORITY_TIME_CHANGED"),
            revalidated,
        ));
    }
    let (profile, manifest_release) =
        match ComputePluginWorkAdmissionLaunchProfile::from_authority_source(
            facts.signed_manifest(),
            facts.signed_manifest_envelope_digest(),
            facts.grant(),
            facts.selected_host_api_revision(),
        ) {
            Ok(value) => value,
            Err(error) => return Err((error, revalidated)),
        };
    if let Err(error) =
        validate_installed_authority_binding(&revalidated, &facts, &manifest_release)
    {
        return Err((error, revalidated));
    }
    let receipts = match build_work_admission_receipts(
        &authority_session,
        &facts,
        revalidated.installed(),
        profile,
    ) {
        Ok(value) => value,
        Err(error) => return Err((error, revalidated)),
    };
    Ok(AuthorizedInstalledWorkAdmission {
        revalidated,
        authority_session,
        facts,
        receipts,
    })
}

fn validate_installed_authority_binding(
    revalidated: &RevalidatedInstalledWorkAdmission<'_>,
    facts: &ComputePluginInstalledWorkAdmissionAuthorityFacts,
    manifest_release: &crate::node_agent_compute_plugin_host::identity::ComputePluginReleaseRef,
) -> anyhow::Result<()> {
    let installed = revalidated.installed();
    installed.receipts().validate()?;
    let install = installed.receipts().install();
    let promotion = installed.receipts().promotion();
    let install_body = install.receipt();
    let promotion_body = promotion.receipt();
    if facts.plugin_id() != install_body.plugin_id()
        || facts.slot_ref() != install_body.slot_ref()
        || facts.release() != install_body.release()
        || facts.release() != manifest_release
        || facts.install_receipt_id() != install_body.install_receipt_id()
        || facts.install_receipt_digest() != install.receipt_digest()
        || facts.promotion_receipt_id() != promotion_body.promotion_receipt_id()
        || facts.promotion_receipt_digest() != promotion.receipt_digest()
        || facts.signed_manifest_envelope_digest() != install_body.signed_manifest_envelope_digest()
        || facts.install_generation() != install_body.install_generation_after()
        || facts.activation_generation() != promotion_body.activation_generation_after()
    {
        anyhow::bail!("COMPUTE_PLUGIN_WORK_ADMISSION_INSTALLED_BINDING_CHANGED");
    }
    Ok(())
}

impl<'permit, 'root> ValidatedInstalledWorkAdmissionStorePermit<'permit, 'root> {
    pub(super) fn new(
        authorized: &'permit AuthorizedInstalledWorkAdmission<'root, 'permit>,
    ) -> Self {
        Self { authorized }
    }

    pub(in crate::node_agent_compute_plugin_host) fn revalidated(
        &self,
    ) -> &RevalidatedInstalledWorkAdmission<'root> {
        &self.authorized.revalidated
    }

    pub(in crate::node_agent_compute_plugin_host) fn facts(
        &self,
    ) -> &ComputePluginInstalledWorkAdmissionAuthorityFacts {
        &self.authorized.facts
    }

    pub(in crate::node_agent_compute_plugin_host) fn receipts(
        &self,
    ) -> &ComputePluginWorkAdmissionReceiptPair {
        &self.authorized.receipts
    }
}

impl AuthorizedInstalledWorkAdmission<'_, '_> {
    pub(super) fn authority_session(
        &self,
    ) -> &ComputePluginPostRevalidationWorkAdmissionAuthoritySession<'_> {
        &self.authority_session
    }

    pub(super) fn facts(&self) -> &ComputePluginInstalledWorkAdmissionAuthorityFacts {
        &self.facts
    }

    pub(super) fn receipts(&self) -> &ComputePluginWorkAdmissionReceiptPair {
        &self.receipts
    }
}

impl<'root> AuthorizedInstalledWorkAdmission<'root, '_> {
    pub(super) fn into_parts(
        self,
    ) -> (
        RevalidatedInstalledWorkAdmission<'root>,
        ComputePluginWorkAdmissionReceiptPair,
    ) {
        (self.revalidated, self.receipts)
    }
}

impl<'root> InstalledWorkAdmissionAuthorizationFailure<'root> {
    pub(in crate::node_agent_compute_plugin_host) fn into_parts(
        self,
    ) -> (Error, RevalidatedInstalledWorkAdmission<'root>) {
        (self.error, self.revalidated)
    }
}

impl fmt::Display for InstalledWorkAdmissionAuthorizationFailure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#}", self.error)
    }
}

impl fmt::Debug for InstalledWorkAdmissionAuthorizationFailure<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InstalledWorkAdmissionAuthorizationFailure")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl StdError for InstalledWorkAdmissionAuthorizationFailure<'_> {}

impl fmt::Debug for AuthorizedInstalledWorkAdmission<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthorizedInstalledWorkAdmission")
            .field("receipt", &"<sealed>")
            .field("facts", &self.facts)
            .finish_non_exhaustive()
    }
}
