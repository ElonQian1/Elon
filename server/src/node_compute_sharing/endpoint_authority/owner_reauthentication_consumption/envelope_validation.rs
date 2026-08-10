use anyhow::{bail, Result};
use serde::Serialize;

use super::super::{
    canonical::{deterministic_identifier, parse_utc_nanos},
    types::{bounded_identifier, is_sha256, safe_positive},
};
use super::contracts::{
    CredentialMutationResultProjection, NodeEndpointOwnerReauthenticationConsumptionEnvelope,
    CONSUMPTION_ID_DOMAIN, CONSUMPTION_SCHEMA,
};

impl CredentialMutationResultProjection {
    pub(super) fn validate(&self) -> Result<()> {
        if !bounded_identifier(&self.current_credential_id, 160)
            || !safe_positive(self.current_credential_revision)
            || !is_sha256(&self.current_credential_digest)
            || !matches!(
                self.current_credential_status.as_str(),
                "active" | "revoked"
            )
        {
            bail!("NODE_ENDPOINT_CREDENTIAL_MUTATION_RESULT_INVALID");
        }
        match (
            &self.issued_credential_id,
            self.issued_credential_revision,
            &self.issued_credential_digest,
        ) {
            (None, None, None) => {}
            (Some(id), Some(revision), Some(digest))
                if id == &self.current_credential_id
                    && revision == self.current_credential_revision
                    && digest == &self.current_credential_digest => {}
            _ => bail!("NODE_ENDPOINT_CREDENTIAL_MUTATION_ISSUED_PROJECTION_INVALID"),
        }
        match (&self.revocation_id, &self.revocation_digest) {
            (None, None) => {}
            (Some(id), Some(digest)) if bounded_identifier(id, 160) && is_sha256(digest) => {}
            _ => bail!("NODE_ENDPOINT_CREDENTIAL_MUTATION_REVOCATION_PROJECTION_INVALID"),
        }
        Ok(())
    }
}

impl NodeEndpointOwnerReauthenticationConsumptionEnvelope {
    pub(super) fn validate(&self) -> Result<()> {
        if self.schema != CONSUMPTION_SCHEMA
            || !bounded_identifier(&self.consumption_id, 160)
            || !bounded_identifier(&self.reauthentication_receipt_id, 160)
            || !is_sha256(&self.reauthentication_digest)
            || !bounded_identifier(&self.owner_user_id, 160)
            || !matches!(
                self.authorization_action.as_str(),
                "initial_registration"
                    | "credential_rotation"
                    | "account_recovery"
                    | "owner_revocation"
            )
            || !bounded_identifier(&self.credential_mutation_request_id, 160)
            || !is_sha256(&self.credential_mutation_request_digest)
            || !is_sha256(&self.authorization_target_digest)
        {
            bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_CONSUMPTION_INVALID");
        }
        self.credential_result.validate()?;
        validate_result_shape(self)?;
        let consumed = parse_utc_nanos(
            &self.consumed_at,
            "NODE_ENDPOINT_REAUTHENTICATION_CONSUMED_AT_INVALID",
        )?;
        let recorded = parse_utc_nanos(
            &self.recorded_at,
            "NODE_ENDPOINT_REAUTHENTICATION_CONSUMPTION_RECORDED_AT_INVALID",
        )?;
        if consumed > recorded {
            bail!("NODE_ENDPOINT_REAUTHENTICATION_CONSUMPTION_TIME_INVALID");
        }
        #[derive(Serialize)]
        struct Identity<'a> {
            reauthentication_receipt_id: &'a str,
            reauthentication_digest: &'a str,
            credential_mutation_request_id: &'a str,
            credential_result: &'a CredentialMutationResultProjection,
        }
        let expected_id = deterministic_identifier(
            "nerconsume_",
            CONSUMPTION_ID_DOMAIN,
            &Identity {
                reauthentication_receipt_id: &self.reauthentication_receipt_id,
                reauthentication_digest: &self.reauthentication_digest,
                credential_mutation_request_id: &self.credential_mutation_request_id,
                credential_result: &self.credential_result,
            },
        )?;
        if expected_id != self.consumption_id {
            bail!("NODE_ENDPOINT_REAUTHENTICATION_CONSUMPTION_ID_MISMATCH");
        }
        Ok(())
    }
}

fn validate_result_shape(
    envelope: &NodeEndpointOwnerReauthenticationConsumptionEnvelope,
) -> Result<()> {
    let result = &envelope.credential_result;
    match envelope.authorization_action.as_str() {
        "initial_registration"
            if result.current_credential_status == "active"
                && result.current_credential_revision == 1
                && result.issued_credential_id.is_some()
                && result.revocation_id.is_none() =>
        {
            Ok(())
        }
        "credential_rotation" | "account_recovery"
            if result.current_credential_status == "active"
                && result.issued_credential_id.is_some()
                && result.revocation_id.is_some() =>
        {
            Ok(())
        }
        "owner_revocation"
            if result.current_credential_status == "revoked"
                && result.issued_credential_id.is_none()
                && result.revocation_id.is_some() =>
        {
            Ok(())
        }
        _ => bail!("NODE_ENDPOINT_REAUTHENTICATION_CONSUMPTION_RESULT_SHAPE_INVALID"),
    }
}
