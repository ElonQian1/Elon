use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;

use super::super::{
    canonical::{canonical_domain_json_and_digest, deterministic_identifier, utc_nanos},
    credential::{
        NodeEndpointCredentialRevocationEnvelope, NodeEndpointCredentialVersionEnvelope,
        PreparedNodeEndpointCredentialRevocation, PreparedNodeEndpointCredentialVersion,
    },
    owner_reauthentication::PreparedNodeEndpointOwnerReauthentication,
    types::NodeEndpointCredentialBinding,
};
use super::contracts::{
    ConsumptionTimes, CredentialMutationResultProjection,
    NodeEndpointCredentialMutationResultBinding,
    NodeEndpointOwnerReauthenticationConsumptionEnvelope,
    PreparedNodeEndpointOwnerReauthenticationConsumption, CONSUMPTION_DIGEST_DOMAIN,
    CONSUMPTION_ID_DOMAIN, CONSUMPTION_SCHEMA,
};

impl NodeEndpointCredentialMutationResultBinding {
    pub(crate) fn from_prepared_mutation(
        current: &NodeEndpointCredentialBinding,
        issued: Option<&PreparedNodeEndpointCredentialVersion>,
        revocation: Option<&PreparedNodeEndpointCredentialRevocation>,
    ) -> Result<Self> {
        let issued = issued
            .map(|value| -> Result<_> {
                value
                    .envelope()
                    .validate_store_readback(value.credential_json(), value.credential_digest())?;
                Ok((
                    value.envelope().clone(),
                    value.credential_digest().to_string(),
                ))
            })
            .transpose()?;
        let revocation = revocation
            .map(|value| -> Result<_> {
                value
                    .envelope()
                    .validate_store_readback(value.revocation_json(), value.revocation_digest())?;
                Ok((
                    value.envelope().clone(),
                    value.revocation_digest().to_string(),
                ))
            })
            .transpose()?;
        Self::seal(current, issued, revocation)
    }

    #[allow(clippy::type_complexity)]
    pub(crate) fn from_store_readback(
        current: &NodeEndpointCredentialBinding,
        issued: Option<(&NodeEndpointCredentialVersionEnvelope, &str, &str)>,
        revocation: Option<(&NodeEndpointCredentialRevocationEnvelope, &str, &str)>,
    ) -> Result<Self> {
        let issued = issued
            .map(|(envelope, stored_json, stored_digest)| -> Result<_> {
                envelope.validate_store_readback(stored_json, stored_digest)?;
                Ok((envelope.clone(), stored_digest.to_string()))
            })
            .transpose()?;
        let revocation = revocation
            .map(|(envelope, stored_json, stored_digest)| -> Result<_> {
                envelope.validate_store_readback(stored_json, stored_digest)?;
                Ok((envelope.clone(), stored_digest.to_string()))
            })
            .transpose()?;
        Self::seal(current, issued, revocation)
    }

    fn seal(
        current: &NodeEndpointCredentialBinding,
        issued: Option<(NodeEndpointCredentialVersionEnvelope, String)>,
        revocation: Option<(NodeEndpointCredentialRevocationEnvelope, String)>,
    ) -> Result<Self> {
        current.validate()?;
        let projection = CredentialMutationResultProjection {
            current_credential_id: current.credential_id().to_string(),
            current_credential_revision: current.credential_revision(),
            current_credential_digest: current.credential_digest().to_string(),
            current_credential_status: current.status().to_string(),
            issued_credential_id: issued
                .as_ref()
                .map(|(envelope, _)| envelope.credential_id().to_string()),
            issued_credential_revision: issued
                .as_ref()
                .map(|(envelope, _)| envelope.credential_revision()),
            issued_credential_digest: issued.as_ref().map(|(_, digest)| digest.clone()),
            revocation_id: revocation
                .as_ref()
                .map(|(envelope, _)| envelope.revocation_id().to_string()),
            revocation_digest: revocation.as_ref().map(|(_, digest)| digest.clone()),
        };
        let binding = Self {
            projection,
            current: current.clone(),
            issued: issued.as_ref().map(|(envelope, _)| envelope.clone()),
            issued_digest: issued.map(|(_, digest)| digest),
            revocation: revocation.as_ref().map(|(envelope, _)| envelope.clone()),
            revocation_digest: revocation.map(|(_, digest)| digest),
        };
        binding.validate_shape()?;
        Ok(binding)
    }
}

pub(crate) fn prepare_owner_reauthentication_consumption(
    reauthentication: &PreparedNodeEndpointOwnerReauthentication,
    credential_result: NodeEndpointCredentialMutationResultBinding,
    consumed_at: DateTime<Utc>,
    recorded_at: DateTime<Utc>,
) -> Result<PreparedNodeEndpointOwnerReauthenticationConsumption> {
    reauthentication.envelope().validate_store_readback(
        reauthentication.receipt_json(),
        reauthentication.receipt_digest(),
    )?;
    let times = ConsumptionTimes {
        consumed_at,
        recorded_at,
    };
    credential_result.validate_against(
        reauthentication.envelope(),
        reauthentication.receipt_digest(),
        &times,
    )?;

    #[derive(Serialize)]
    struct Identity<'a> {
        reauthentication_receipt_id: &'a str,
        reauthentication_digest: &'a str,
        credential_mutation_request_id: &'a str,
        credential_result: &'a CredentialMutationResultProjection,
    }
    let source = reauthentication.envelope();
    let consumption_id = deterministic_identifier(
        "nerconsume_",
        CONSUMPTION_ID_DOMAIN,
        &Identity {
            reauthentication_receipt_id: source.reauthentication_receipt_id(),
            reauthentication_digest: reauthentication.receipt_digest(),
            credential_mutation_request_id: source.credential_mutation_request_id(),
            credential_result: &credential_result.projection,
        },
    )?;
    let envelope = NodeEndpointOwnerReauthenticationConsumptionEnvelope {
        schema: CONSUMPTION_SCHEMA.to_string(),
        consumption_id,
        reauthentication_receipt_id: source.reauthentication_receipt_id().to_string(),
        reauthentication_digest: reauthentication.receipt_digest().to_string(),
        owner_user_id: source.owner_user_id().to_string(),
        authorization_action: source.authorization_action().to_string(),
        credential_mutation_request_id: source.credential_mutation_request_id().to_string(),
        credential_mutation_request_digest: source.credential_mutation_request_digest().to_string(),
        authorization_target_digest: source.authorization_target_digest().to_string(),
        credential_result: credential_result.projection,
        consumed_at: utc_nanos(consumed_at),
        recorded_at: utc_nanos(recorded_at),
    };
    envelope.validate()?;
    let (consumption_json, consumption_digest) =
        canonical_domain_json_and_digest(CONSUMPTION_DIGEST_DOMAIN, &envelope)?;
    Ok(PreparedNodeEndpointOwnerReauthenticationConsumption {
        envelope,
        consumption_json,
        consumption_digest,
    })
}
