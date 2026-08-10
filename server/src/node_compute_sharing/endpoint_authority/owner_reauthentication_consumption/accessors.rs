use anyhow::Result;

use super::super::canonical::ensure_canonical_readback;
use super::contracts::{
    NodeEndpointOwnerReauthenticationConsumptionEnvelope, CONSUMPTION_DIGEST_DOMAIN,
};

impl NodeEndpointOwnerReauthenticationConsumptionEnvelope {
    pub(crate) fn from_store_readback(
        consumption_json: &str,
        consumption_digest: &str,
    ) -> Result<Self> {
        let envelope: Self = serde_json::from_str(consumption_json)?;
        envelope.validate()?;
        ensure_canonical_readback(
            CONSUMPTION_DIGEST_DOMAIN,
            &envelope,
            consumption_json,
            consumption_digest,
        )?;
        Ok(envelope)
    }

    pub(crate) fn validate_store_readback(
        &self,
        consumption_json: &str,
        consumption_digest: &str,
    ) -> Result<()> {
        self.validate()?;
        ensure_canonical_readback(
            CONSUMPTION_DIGEST_DOMAIN,
            self,
            consumption_json,
            consumption_digest,
        )
    }

    pub(crate) fn schema(&self) -> &str {
        &self.schema
    }
    pub(crate) fn consumption_id(&self) -> &str {
        &self.consumption_id
    }
    pub(crate) fn reauthentication_receipt_id(&self) -> &str {
        &self.reauthentication_receipt_id
    }
    pub(crate) fn reauthentication_digest(&self) -> &str {
        &self.reauthentication_digest
    }
    pub(crate) fn owner_user_id(&self) -> &str {
        &self.owner_user_id
    }
    pub(crate) fn authorization_action(&self) -> &str {
        &self.authorization_action
    }
    pub(crate) fn credential_mutation_request_id(&self) -> &str {
        &self.credential_mutation_request_id
    }
    pub(crate) fn credential_mutation_request_digest(&self) -> &str {
        &self.credential_mutation_request_digest
    }
    pub(crate) fn authorization_target_digest(&self) -> &str {
        &self.authorization_target_digest
    }
    pub(crate) fn current_credential_id(&self) -> &str {
        &self.credential_result.current_credential_id
    }
    pub(crate) fn current_credential_revision(&self) -> u64 {
        self.credential_result.current_credential_revision
    }
    pub(crate) fn current_credential_digest(&self) -> &str {
        &self.credential_result.current_credential_digest
    }
    pub(crate) fn current_credential_status(&self) -> &str {
        &self.credential_result.current_credential_status
    }
    pub(crate) fn issued_credential_id(&self) -> Option<&str> {
        self.credential_result.issued_credential_id.as_deref()
    }
    pub(crate) fn issued_credential_revision(&self) -> Option<u64> {
        self.credential_result.issued_credential_revision
    }
    pub(crate) fn issued_credential_digest(&self) -> Option<&str> {
        self.credential_result.issued_credential_digest.as_deref()
    }
    pub(crate) fn revocation_id(&self) -> Option<&str> {
        self.credential_result.revocation_id.as_deref()
    }
    pub(crate) fn revocation_digest(&self) -> Option<&str> {
        self.credential_result.revocation_digest.as_deref()
    }
    pub(crate) fn consumed_at(&self) -> &str {
        &self.consumed_at
    }
    pub(crate) fn recorded_at(&self) -> &str {
        &self.recorded_at
    }
}
