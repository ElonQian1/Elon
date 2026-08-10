use super::*;

impl NodeEndpointOwnerReauthenticationEnvelope {
    pub(crate) fn from_store_readback(receipt_json: &str, receipt_digest: &str) -> Result<Self> {
        let envelope: Self = serde_json::from_str(receipt_json)?;
        envelope.validate()?;
        ensure_canonical_readback(
            OWNER_REAUTHENTICATION_DIGEST_DOMAIN,
            &envelope,
            receipt_json,
            receipt_digest,
        )?;
        Ok(envelope)
    }

    pub(crate) fn validate_store_readback(
        &self,
        receipt_json: &str,
        receipt_digest: &str,
    ) -> Result<()> {
        self.validate()?;
        ensure_canonical_readback(
            OWNER_REAUTHENTICATION_DIGEST_DOMAIN,
            self,
            receipt_json,
            receipt_digest,
        )
    }

    pub(crate) fn schema(&self) -> &str {
        &self.schema
    }
    pub(crate) fn reauthentication_receipt_id(&self) -> &str {
        &self.reauthentication_receipt_id
    }
    pub(crate) fn owner_user_id(&self) -> &str {
        &self.owner_user_id
    }
    pub(crate) fn account_session_id(&self) -> &str {
        &self.account_session_id
    }
    pub(crate) fn session_binding_digest(&self) -> &str {
        &self.session_binding_digest
    }
    pub(crate) fn account_auth_state_digest(&self) -> &str {
        &self.account_auth_state_digest
    }
    pub(crate) fn authentication_method(&self) -> &str {
        &self.authentication_method
    }
    pub(crate) fn authentication_factor_id(&self) -> &str {
        &self.authentication_factor_id
    }
    pub(crate) fn authentication_factor_binding_digest(&self) -> &str {
        &self.authentication_factor_binding_digest
    }
    pub(crate) fn authentication_evidence_id(&self) -> &str {
        &self.authentication_evidence_id
    }
    pub(crate) fn authentication_evidence_digest(&self) -> &str {
        &self.authentication_evidence_digest
    }
    pub(crate) fn authorization_issuance_request_id(&self) -> &str {
        &self.authorization_issuance_request_id
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
    pub(crate) fn agent_id(&self) -> &str {
        &self.agent_id
    }
    pub(crate) fn install_id(&self) -> &str {
        &self.install_id
    }
    pub(crate) fn expected_credential_id(&self) -> Option<&str> {
        self.expected_credential_id.as_deref()
    }
    pub(crate) fn expected_credential_revision(&self) -> Option<u64> {
        self.expected_credential_revision
    }
    pub(crate) fn expected_credential_digest(&self) -> Option<&str> {
        self.expected_credential_digest.as_deref()
    }
    pub(crate) fn secure_transport_source(&self) -> &str {
        &self.secure_transport_source
    }
    pub(crate) fn secure_transport_evidence_schema(&self) -> &str {
        &self.secure_transport_evidence_schema
    }
    pub(crate) fn secure_transport_evidence_id(&self) -> &str {
        &self.secure_transport_evidence_id
    }
    pub(crate) fn secure_transport_evidence_digest(&self) -> &str {
        &self.secure_transport_evidence_digest
    }
    pub(crate) fn secure_transport_verifier_revision(&self) -> u64 {
        self.secure_transport_verifier_revision
    }
    pub(crate) fn secure_transport_verifier_digest(&self) -> &str {
        &self.secure_transport_verifier_digest
    }
    pub(crate) fn secure_transport_server_instance_id(&self) -> &str {
        &self.secure_transport_server_instance_id
    }
    pub(crate) fn secure_transport_request_binding_digest(&self) -> &str {
        &self.secure_transport_request_binding_digest
    }
    pub(crate) fn secure_transport_verified_at(&self) -> &str {
        &self.secure_transport_verified_at
    }
    pub(crate) fn reauthenticated_at(&self) -> &str {
        &self.reauthenticated_at
    }
    pub(crate) fn expires_at(&self) -> &str {
        &self.expires_at
    }
    pub(crate) fn recorded_at(&self) -> &str {
        &self.recorded_at
    }

    pub(super) fn validate(&self) -> Result<()> {
        if self.schema != OWNER_REAUTHENTICATION_SCHEMA
            || !bounded_identifier(&self.reauthentication_receipt_id, 160)
            || !bounded_identifier(&self.owner_user_id, 160)
            || !bounded_identifier(&self.account_session_id, 160)
            || !is_sha256(&self.session_binding_digest)
            || !is_sha256(&self.account_auth_state_digest)
            || !matches!(
                self.authentication_method.as_str(),
                "password" | "google_oidc"
            )
            || !bounded_identifier(&self.authentication_factor_id, 160)
            || !is_sha256(&self.authentication_factor_binding_digest)
            || !bounded_identifier(&self.authentication_evidence_id, 160)
            || !is_sha256(&self.authentication_evidence_digest)
            || !bounded_identifier(&self.authorization_issuance_request_id, 160)
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
            || !bounded_identifier(&self.agent_id, 160)
            || !bounded_identifier(&self.install_id, 512)
            || !matches!(
                self.secure_transport_source.as_str(),
                "direct_tls" | "trusted_proxy_mtls"
            )
            || !bounded_identifier(&self.secure_transport_evidence_schema, 160)
            || !bounded_identifier(&self.secure_transport_evidence_id, 160)
            || !is_sha256(&self.secure_transport_evidence_digest)
            || self.secure_transport_verifier_revision == 0
            || self.secure_transport_verifier_revision
                > super::super::super::types::MAX_IJSON_SAFE_INTEGER
            || !is_sha256(&self.secure_transport_verifier_digest)
            || !bounded_identifier(&self.secure_transport_server_instance_id, 160)
            || !is_sha256(&self.secure_transport_request_binding_digest)
            || self.secure_transport_request_binding_digest
                != self.credential_mutation_request_digest
        {
            bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_ENVELOPE_INVALID");
        }
        match (
            &self.expected_credential_id,
            self.expected_credential_revision,
            &self.expected_credential_digest,
        ) {
            (None, None, None) if self.authorization_action == "initial_registration" => {}
            (Some(id), Some(revision), Some(digest))
                if self.authorization_action != "initial_registration"
                    && bounded_identifier(id, 160)
                    && revision > 0
                    && revision <= super::super::super::types::MAX_IJSON_SAFE_INTEGER
                    && is_sha256(digest) => {}
            _ => bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_EXPECTED_BINDING_INVALID"),
        }
        let secure = parse_utc_nanos(
            &self.secure_transport_verified_at,
            "NODE_ENDPOINT_OWNER_TRANSPORT_TIMESTAMP_INVALID",
        )?;
        let reauthenticated = parse_utc_nanos(
            &self.reauthenticated_at,
            "NODE_ENDPOINT_OWNER_REAUTHENTICATED_TIMESTAMP_INVALID",
        )?;
        let expires = parse_utc_nanos(
            &self.expires_at,
            "NODE_ENDPOINT_OWNER_REAUTHENTICATION_EXPIRY_INVALID",
        )?;
        let recorded = parse_utc_nanos(
            &self.recorded_at,
            "NODE_ENDPOINT_OWNER_REAUTHENTICATION_RECORDED_AT_INVALID",
        )?;
        if secure > reauthenticated
            || secure
                .checked_add_signed(Duration::seconds(MAX_TRANSPORT_TO_REAUTH_SECONDS))
                .is_none_or(|deadline| reauthenticated > deadline)
            || reauthenticated
                .checked_add_signed(Duration::minutes(REAUTHENTICATION_LIFETIME_MINUTES))
                != Some(expires)
            || recorded < reauthenticated
            || recorded >= expires
        {
            bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_TIME_INVALID");
        }
        let expected_id = deterministic_identifier(
            "nerauth_",
            OWNER_REAUTHENTICATION_ID_DOMAIN,
            &serde_json::json!({
                "owner_user_id": self.owner_user_id,
                "authorization_issuance_request_id": self.authorization_issuance_request_id,
                "authentication_evidence_digest": self.authentication_evidence_digest,
                "authorization_target_digest": self.authorization_target_digest,
            }),
        )?;
        if expected_id != self.reauthentication_receipt_id {
            bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_ID_MISMATCH");
        }
        let expected_target_digest = authorization_target_digest_from_parts(
            &self.authorization_action,
            &self.owner_user_id,
            &self.agent_id,
            &self.install_id,
            self.expected_credential_id.as_deref(),
            self.expected_credential_revision,
            self.expected_credential_digest.as_deref(),
            &self.credential_mutation_request_id,
            &self.credential_mutation_request_digest,
        )?;
        if expected_target_digest != self.authorization_target_digest {
            bail!("NODE_ENDPOINT_OWNER_REAUTHENTICATION_TARGET_DIGEST_MISMATCH");
        }
        Ok(())
    }
}
