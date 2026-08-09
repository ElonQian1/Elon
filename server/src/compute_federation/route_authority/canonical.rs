use anyhow::Result;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_plugin_sharing_directive::canonical_compute_plugin_ijson_and_sha256;

use super::types::{
    ComputeRouteAdapterVersion, ComputeRouteAdapterVersionEnvelope, ComputeRouteAuthorization,
    ComputeRouteAuthorizationEnvelope, ComputeRouteAuthorizationSealEnvelope,
    ComputeRouteCapabilityBinding, ComputeRouteCredential, ComputeRouteCredentialEnvelope,
    ComputeRouteCredentialRevocationEnvelope, ComputeServiceActorAuthorization,
    ComputeServiceActorAuthorizationEnvelope,
};

const MAX_ROUTE_AUTHORITY_JSON_BYTES: usize = 2 * 1024 * 1024;
const ADAPTER_VERSION_DOMAIN: &[u8] = b"ELON-COMPUTE-ROUTE-ADAPTER-VERSION-V1";
const CREDENTIAL_DOMAIN: &[u8] = b"ELON-COMPUTE-ROUTE-CREDENTIAL-V1";
const CREDENTIAL_REVOCATION_DOMAIN: &[u8] = b"ELON-COMPUTE-ROUTE-CREDENTIAL-REVOCATION-V1";
const ROUTE_AUTHORIZATION_DOMAIN: &[u8] = b"ELON-COMPUTE-ROUTE-AUTHORIZATION-V1";
const ROUTE_AUTHORIZATION_SEAL_DOMAIN: &[u8] = b"ELON-COMPUTE-ROUTE-AUTHORIZATION-SEAL-V1";
const ROUTE_CAPABILITY_SET_DOMAIN: &[u8] = b"ELON-COMPUTE-ROUTE-CAPABILITY-SET-V1";
const SERVICE_ACTOR_AUTHORIZATION_DOMAIN: &[u8] = b"ELON-COMPUTE-SERVICE-ACTOR-AUTHORIZATION-V1";

pub(crate) fn canonical_route_adapter_version_json_and_digest(
    envelope: &ComputeRouteAdapterVersionEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct Projection<'a> {
        schema: &'a str,
        adapter_id: &'a str,
        adapter_revision: i64,
        canonicalization: &'a str,
        digest_algorithm: &'a str,
        adapter: &'a ComputeRouteAdapterVersion,
    }
    envelope_json_and_digest(
        ADAPTER_VERSION_DOMAIN,
        &Projection {
            schema: &envelope.schema,
            adapter_id: &envelope.adapter_id,
            adapter_revision: envelope.adapter_revision,
            canonicalization: &envelope.canonicalization,
            digest_algorithm: &envelope.digest_algorithm,
            adapter: &envelope.adapter,
        },
        envelope,
    )
}

pub(crate) fn canonical_route_credential_json_and_digest(
    envelope: &ComputeRouteCredentialEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct Projection<'a> {
        schema: &'a str,
        credential_id: &'a str,
        credential_revision: i64,
        canonicalization: &'a str,
        digest_algorithm: &'a str,
        credential: &'a ComputeRouteCredential,
    }
    envelope_json_and_digest(
        CREDENTIAL_DOMAIN,
        &Projection {
            schema: &envelope.schema,
            credential_id: &envelope.credential_id,
            credential_revision: envelope.credential_revision,
            canonicalization: &envelope.canonicalization,
            digest_algorithm: &envelope.digest_algorithm,
            credential: &envelope.credential,
        },
        envelope,
    )
}

pub(crate) fn canonical_route_credential_revocation_json_and_digest(
    envelope: &ComputeRouteCredentialRevocationEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct Projection<'a> {
        schema: &'a str,
        revocation_id: &'a str,
        canonicalization: &'a str,
        digest_algorithm: &'a str,
        credential_id: &'a str,
        credential_revision: i64,
        credential_digest: &'a str,
        provider_id: &'a str,
        reason_code: &'a str,
        revoked_by_service_actor_id: &'a str,
        actor_authorization_id: &'a str,
        actor_authorization_digest: &'a str,
        revoked_at: &'a str,
        recorded_at: &'a str,
    }
    envelope_json_and_digest(
        CREDENTIAL_REVOCATION_DOMAIN,
        &Projection {
            schema: &envelope.schema,
            revocation_id: &envelope.revocation_id,
            canonicalization: &envelope.canonicalization,
            digest_algorithm: &envelope.digest_algorithm,
            credential_id: &envelope.credential_id,
            credential_revision: envelope.credential_revision,
            credential_digest: &envelope.credential_digest,
            provider_id: &envelope.provider_id,
            reason_code: &envelope.reason_code,
            revoked_by_service_actor_id: &envelope.revoked_by_service_actor_id,
            actor_authorization_id: &envelope.actor_authorization_id,
            actor_authorization_digest: &envelope.actor_authorization_digest,
            revoked_at: &envelope.revoked_at,
            recorded_at: &envelope.recorded_at,
        },
        envelope,
    )
}

pub(crate) fn canonical_route_authorization_json_and_digest(
    envelope: &ComputeRouteAuthorizationEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct Projection<'a> {
        schema: &'a str,
        route_authorization_id: &'a str,
        route_authorization_revision: i64,
        canonicalization: &'a str,
        digest_algorithm: &'a str,
        authorization: &'a ComputeRouteAuthorization,
    }
    envelope_json_and_digest(
        ROUTE_AUTHORIZATION_DOMAIN,
        &Projection {
            schema: &envelope.schema,
            route_authorization_id: &envelope.route_authorization_id,
            route_authorization_revision: envelope.route_authorization_revision,
            canonicalization: &envelope.canonicalization,
            digest_algorithm: &envelope.digest_algorithm,
            authorization: &envelope.authorization,
        },
        envelope,
    )
}

pub(crate) fn canonical_route_capability_set_digest(
    capabilities: &[ComputeRouteCapabilityBinding],
) -> Result<String> {
    domain_digest(ROUTE_CAPABILITY_SET_DOMAIN, capabilities)
}

pub(crate) fn canonical_route_authorization_seal_json_and_digest(
    seal: &ComputeRouteAuthorizationSealEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct Projection<'a> {
        schema: &'a str,
        seal_id: &'a str,
        canonicalization: &'a str,
        digest_algorithm: &'a str,
        route_authorization_id: &'a str,
        route_authorization_revision: i64,
        route_authorization_digest: &'a str,
        adapter_id: &'a str,
        adapter_revision: i64,
        adapter_registry_digest: &'a str,
        credential_id: &'a str,
        credential_revision: i64,
        credential_digest: &'a str,
        capability_count: i64,
        capability_set_digest: &'a str,
        sealed_at: &'a str,
    }
    envelope_json_and_digest(
        ROUTE_AUTHORIZATION_SEAL_DOMAIN,
        &Projection {
            schema: &seal.schema,
            seal_id: &seal.seal_id,
            canonicalization: &seal.canonicalization,
            digest_algorithm: &seal.digest_algorithm,
            route_authorization_id: &seal.route_authorization_id,
            route_authorization_revision: seal.route_authorization_revision,
            route_authorization_digest: &seal.route_authorization_digest,
            adapter_id: &seal.adapter_id,
            adapter_revision: seal.adapter_revision,
            adapter_registry_digest: &seal.adapter_registry_digest,
            credential_id: &seal.credential_id,
            credential_revision: seal.credential_revision,
            credential_digest: &seal.credential_digest,
            capability_count: seal.capability_count,
            capability_set_digest: &seal.capability_set_digest,
            sealed_at: &seal.sealed_at,
        },
        seal,
    )
}

pub(crate) fn canonical_service_actor_authorization_json_and_digest(
    envelope: &ComputeServiceActorAuthorizationEnvelope,
) -> Result<(String, String)> {
    #[derive(Serialize)]
    struct Projection<'a> {
        schema: &'a str,
        actor_authorization_id: &'a str,
        actor_authorization_revision: i64,
        canonicalization: &'a str,
        digest_algorithm: &'a str,
        authorization: &'a ComputeServiceActorAuthorization,
    }
    envelope_json_and_digest(
        SERVICE_ACTOR_AUTHORIZATION_DOMAIN,
        &Projection {
            schema: &envelope.schema,
            actor_authorization_id: &envelope.actor_authorization_id,
            actor_authorization_revision: envelope.actor_authorization_revision,
            canonicalization: &envelope.canonicalization,
            digest_algorithm: &envelope.digest_algorithm,
            authorization: &envelope.authorization,
        },
        envelope,
    )
}

fn envelope_json_and_digest<P: Serialize, E: Serialize>(
    domain: &[u8],
    projection: &P,
    envelope: &E,
) -> Result<(String, String)> {
    let digest = domain_digest(domain, projection)?;
    let (json, _) =
        canonical_compute_plugin_ijson_and_sha256(envelope, MAX_ROUTE_AUTHORITY_JSON_BYTES)?;
    Ok((json, digest))
}

fn domain_digest<T: Serialize>(domain: &[u8], value: &T) -> Result<String> {
    let (json, _) =
        canonical_compute_plugin_ijson_and_sha256(value, MAX_ROUTE_AUTHORITY_JSON_BYTES)?;
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update([0]);
    digest.update(json.as_bytes());
    Ok(hex::encode(digest.finalize()))
}
