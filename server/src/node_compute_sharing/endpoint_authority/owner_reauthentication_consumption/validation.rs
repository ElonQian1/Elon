use super::super::{
    canonical::{parse_utc_nanos, utc_nanos},
    owner_reauthentication::NodeEndpointOwnerReauthenticationEnvelope,
    types::is_sha256,
};
use super::contracts::{ConsumptionTimes, NodeEndpointCredentialMutationResultBinding};
use anyhow::{bail, Result};

impl NodeEndpointCredentialMutationResultBinding {
    pub(super) fn validate_shape(&self) -> Result<()> {
        self.current.validate()?;
        self.projection.validate()?;
        if self.projection.current_credential_id != self.current.credential_id()
            || self.projection.current_credential_revision != self.current.credential_revision()
            || self.projection.current_credential_digest != self.current.credential_digest()
            || self.projection.current_credential_status != self.current.status()
            || self.issued.is_some() != self.issued_digest.is_some()
            || self.revocation.is_some() != self.revocation_digest.is_some()
        {
            bail!("NODE_ENDPOINT_CREDENTIAL_MUTATION_RESULT_BINDING_MISMATCH");
        }
        match (&self.issued, &self.issued_digest) {
            (Some(issued), Some(digest))
                if self.current.status() == "active"
                    && issued.credential_id() == self.current.credential_id()
                    && issued.credential_revision() == self.current.credential_revision()
                    && digest == self.current.credential_digest()
                    && issued.agent_id() == self.current.agent_id()
                    && issued.owner_user_id() == self.current.owner_user_id()
                    && issued.install_id() == self.current.install_id()
                    && issued.installation_binding_digest()
                        == self.current.installation_binding_digest() => {}
            (None, None) if self.current.status() == "revoked" => {}
            _ => bail!("NODE_ENDPOINT_CREDENTIAL_MUTATION_ISSUED_RESULT_INVALID"),
        }
        match (&self.revocation, &self.revocation_digest) {
            (Some(revocation), Some(digest))
                if revocation.credential_id() == self.current.credential_id()
                    && revocation.agent_id() == self.current.agent_id()
                    && revocation.owner_user_id() == self.current.owner_user_id()
                    && is_sha256(digest)
                    && ((self.current.status() == "active"
                        && revocation.credential_revision().checked_add(1)
                            == Some(self.current.credential_revision()))
                        || (self.current.status() == "revoked"
                            && revocation.credential_revision()
                                == self.current.credential_revision()
                            && revocation.credential_digest()
                                == self.current.credential_digest())) => {}
            (None, None) if self.current.status() == "active" => {}
            _ => bail!("NODE_ENDPOINT_CREDENTIAL_MUTATION_REVOCATION_RESULT_INVALID"),
        }
        Ok(())
    }

    pub(super) fn validate_against(
        &self,
        source: &NodeEndpointOwnerReauthenticationEnvelope,
        source_digest: &str,
        times: &ConsumptionTimes,
    ) -> Result<()> {
        self.validate_shape()?;
        validate_consumption_times(source, times)?;
        if self.current.owner_user_id() != source.owner_user_id()
            || self.current.agent_id() != source.agent_id()
            || self.current.install_id() != source.install_id()
        {
            bail!("NODE_ENDPOINT_REAUTHENTICATION_RESULT_TARGET_MISMATCH");
        }
        match source.authorization_action() {
            "initial_registration" => self.validate_initial(source, source_digest, times),
            "credential_rotation" => {
                self.validate_successor(source, source_digest, times, "credential_rotation")
            }
            "account_recovery" => {
                self.validate_successor(source, source_digest, times, "account_recovery")
            }
            "owner_revocation" => self.validate_owner_revocation(source, source_digest, times),
            _ => bail!("NODE_ENDPOINT_REAUTHENTICATION_RESULT_ACTION_INVALID"),
        }
    }

    fn validate_initial(
        &self,
        source: &NodeEndpointOwnerReauthenticationEnvelope,
        source_digest: &str,
        times: &ConsumptionTimes,
    ) -> Result<()> {
        let issued = self.issued.as_ref().ok_or_else(|| {
            anyhow::anyhow!("NODE_ENDPOINT_REAUTHENTICATION_INITIAL_RESULT_MISSING")
        })?;
        if source.expected_credential_id().is_some()
            || self.current.status() != "active"
            || self.current.credential_revision() != 1
            || self.revocation.is_some()
            || issued.issuance_kind() != "initial_registration"
            || issued.previous_credential_revision().is_some()
            || issued.previous_credential_digest().is_some()
        {
            bail!("NODE_ENDPOINT_REAUTHENTICATION_INITIAL_RESULT_INVALID");
        }
        validate_issued_source(issued, source, source_digest, times)
    }

    fn validate_successor(
        &self,
        source: &NodeEndpointOwnerReauthenticationEnvelope,
        source_digest: &str,
        times: &ConsumptionTimes,
        issuance_kind: &str,
    ) -> Result<()> {
        let issued = self.issued.as_ref().ok_or_else(|| {
            anyhow::anyhow!("NODE_ENDPOINT_REAUTHENTICATION_SUCCESSOR_RESULT_MISSING")
        })?;
        let revocation = self.revocation.as_ref().ok_or_else(|| {
            anyhow::anyhow!("NODE_ENDPOINT_REAUTHENTICATION_REVOCATION_RESULT_MISSING")
        })?;
        let expected_id = source.expected_credential_id().ok_or_else(|| {
            anyhow::anyhow!("NODE_ENDPOINT_REAUTHENTICATION_EXPECTED_RESULT_MISSING")
        })?;
        let expected_revision = source.expected_credential_revision().ok_or_else(|| {
            anyhow::anyhow!("NODE_ENDPOINT_REAUTHENTICATION_EXPECTED_RESULT_MISSING")
        })?;
        let expected_digest = source.expected_credential_digest().ok_or_else(|| {
            anyhow::anyhow!("NODE_ENDPOINT_REAUTHENTICATION_EXPECTED_RESULT_MISSING")
        })?;
        if issued.issuance_kind() != issuance_kind
            || issued.credential_id() != expected_id
            || issued.previous_credential_revision() != Some(expected_revision)
            || issued.previous_credential_digest() != Some(expected_digest)
            || expected_revision.checked_add(1) != Some(issued.credential_revision())
            || revocation.credential_id() != expected_id
            || revocation.credential_revision() != expected_revision
            || revocation.credential_digest() != expected_digest
        {
            bail!("NODE_ENDPOINT_REAUTHENTICATION_SUCCESSOR_RESULT_INVALID");
        }
        validate_issued_source(issued, source, source_digest, times)?;
        match (issuance_kind, revocation.revocation_kind()) {
            ("credential_rotation", "rotated") | ("account_recovery", "recovered") => {
                validate_new_revocation_source(revocation, source, source_digest, times)
            }
            ("account_recovery", "owner_revoked" | "security_revoked") => {
                let terminal_recorded = parse_utc_nanos(
                    revocation.recorded_at(),
                    "NODE_ENDPOINT_RECOVERY_TERMINAL_REVOCATION_TIME_INVALID",
                )?;
                if terminal_recorded > times.consumed_at {
                    bail!("NODE_ENDPOINT_RECOVERY_TERMINAL_REVOCATION_IN_FUTURE");
                }
                Ok(())
            }
            _ => bail!("NODE_ENDPOINT_REAUTHENTICATION_SUCCESSOR_REVOCATION_INVALID"),
        }
    }

    fn validate_owner_revocation(
        &self,
        source: &NodeEndpointOwnerReauthenticationEnvelope,
        source_digest: &str,
        times: &ConsumptionTimes,
    ) -> Result<()> {
        let revocation = self.revocation.as_ref().ok_or_else(|| {
            anyhow::anyhow!("NODE_ENDPOINT_REAUTHENTICATION_OWNER_REVOCATION_MISSING")
        })?;
        if self.current.status() != "revoked"
            || self.issued.is_some()
            || source.expected_credential_id() != Some(self.current.credential_id())
            || source.expected_credential_revision() != Some(self.current.credential_revision())
            || source.expected_credential_digest() != Some(self.current.credential_digest())
            || revocation.revocation_kind() != "owner_revoked"
        {
            bail!("NODE_ENDPOINT_REAUTHENTICATION_OWNER_REVOCATION_INVALID");
        }
        validate_new_revocation_source(revocation, source, source_digest, times)
    }
}

fn validate_issued_source(
    issued: &super::super::credential::NodeEndpointCredentialVersionEnvelope,
    source: &NodeEndpointOwnerReauthenticationEnvelope,
    source_digest: &str,
    times: &ConsumptionTimes,
) -> Result<()> {
    if issued.agent_id() != source.agent_id()
        || issued.owner_user_id() != source.owner_user_id()
        || issued.install_id() != source.install_id()
        || issued.issuance_request_id() != source.credential_mutation_request_id()
        || issued.issued_by_user_id() != source.owner_user_id()
        || issued.issued_at() != utc_nanos(times.consumed_at)
        || issued.recorded_at() != utc_nanos(times.recorded_at)
    {
        bail!("NODE_ENDPOINT_REAUTHENTICATION_ISSUED_SOURCE_MISMATCH");
    }
    validate_recent_basis(
        issued.owner_authorization_basis(),
        source.reauthentication_receipt_id(),
        source_digest,
    )
}

fn validate_new_revocation_source(
    revocation: &super::super::credential::NodeEndpointCredentialRevocationEnvelope,
    source: &NodeEndpointOwnerReauthenticationEnvelope,
    source_digest: &str,
    times: &ConsumptionTimes,
) -> Result<()> {
    if revocation.agent_id() != source.agent_id()
        || revocation.owner_user_id() != source.owner_user_id()
        || revocation.mutation_request_id() != source.credential_mutation_request_id()
        || revocation.revoked_by_user_id() != source.owner_user_id()
        || revocation.revoked_at() != utc_nanos(times.consumed_at)
        || revocation.recorded_at() != utc_nanos(times.recorded_at)
    {
        bail!("NODE_ENDPOINT_REAUTHENTICATION_REVOCATION_SOURCE_MISMATCH");
    }
    validate_recent_basis(
        revocation.owner_authorization_basis(),
        source.reauthentication_receipt_id(),
        source_digest,
    )
}

fn validate_recent_basis(
    basis: &super::super::types::NodeEndpointOwnerAuthorizationBasis,
    receipt_id: &str,
    receipt_digest: &str,
) -> Result<()> {
    if basis.kind() != "recent_reauthentication"
        || basis.basis_id() != receipt_id
        || basis.basis_digest() != receipt_digest
    {
        bail!("NODE_ENDPOINT_REAUTHENTICATION_RESULT_BASIS_MISMATCH");
    }
    Ok(())
}

fn validate_consumption_times(
    source: &NodeEndpointOwnerReauthenticationEnvelope,
    times: &ConsumptionTimes,
) -> Result<()> {
    let source_recorded = parse_utc_nanos(
        source.recorded_at(),
        "NODE_ENDPOINT_REAUTHENTICATION_SOURCE_RECORDED_AT_INVALID",
    )?;
    let source_expires = parse_utc_nanos(
        source.expires_at(),
        "NODE_ENDPOINT_REAUTHENTICATION_SOURCE_EXPIRY_INVALID",
    )?;
    if source_recorded > times.consumed_at
        || times.consumed_at > times.recorded_at
        || times.recorded_at >= source_expires
    {
        bail!("NODE_ENDPOINT_REAUTHENTICATION_CONSUMPTION_TIME_INVALID");
    }
    Ok(())
}
