use anyhow::{bail, Result};
use homecli_proto::{
    CAP_NODE_ENDPOINT_PLANNING_SNAPSHOT_BOOTSTRAP_V1, NODE_ENDPOINT_SESSION_V1_PROTO_VERSION,
    NODE_ENDPOINT_SESSION_V2_PROTO_VERSION,
};

use super::{
    bounded_identifier, canonical_node_endpoint_capability_set, is_sha256, safe_positive,
    NodeEndpointSessionOpenRequest, PresentedNodeEndpointCredentialSecret,
};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum NodeEndpointSessionProfile {
    AuthOnlyV13,
    PlanningSnapshotBootstrapV14,
}

impl NodeEndpointSessionProfile {
    pub(crate) fn protocol_version(self) -> u64 {
        match self {
            Self::AuthOnlyV13 => u64::from(NODE_ENDPOINT_SESSION_V1_PROTO_VERSION),
            Self::PlanningSnapshotBootstrapV14 => u64::from(NODE_ENDPOINT_SESSION_V2_PROTO_VERSION),
        }
    }

    pub(crate) fn capabilities(self) -> Vec<String> {
        match self {
            Self::AuthOnlyV13 => Vec::new(),
            Self::PlanningSnapshotBootstrapV14 => {
                vec![CAP_NODE_ENDPOINT_PLANNING_SNAPSHOT_BOOTSTRAP_V1.to_string()]
            }
        }
    }

    pub(crate) fn require_planning_bootstrap_v14(self) -> Result<()> {
        if self != Self::PlanningSnapshotBootstrapV14 {
            bail!("NODE_ENDPOINT_SESSION_PLANNING_PROFILE_REQUIRED");
        }
        Ok(())
    }

    pub(crate) fn from_receipt(
        protocol_version: u64,
        capability_count: u64,
        capability_set_digest: &str,
    ) -> Result<Self> {
        for profile in [Self::AuthOnlyV13, Self::PlanningSnapshotBootstrapV14] {
            let capabilities = profile.capabilities();
            let (_, expected_digest) = canonical_node_endpoint_capability_set(&capabilities)?;
            if protocol_version == profile.protocol_version()
                && capability_count == capabilities.len() as u64
                && capability_set_digest == expected_digest
            {
                return Ok(profile);
            }
        }
        bail!("NODE_ENDPOINT_SESSION_PROFILE_INVALID")
    }
}

impl NodeEndpointSessionOpenRequest {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        agent_id: String,
        owner_user_id: String,
        install_id: String,
        credential_id: String,
        credential_revision: u64,
        credential_digest: String,
        session_id: String,
        server_instance_id: String,
        protocol_version: u64,
        agent_version: String,
        presented_secret: String,
    ) -> Result<Self> {
        if protocol_version != u64::from(NODE_ENDPOINT_SESSION_V1_PROTO_VERSION) {
            bail!("NODE_ENDPOINT_SESSION_OPEN_REQUEST_PROFILE_INVALID");
        }
        Self::new_for_profile(
            NodeEndpointSessionProfile::AuthOnlyV13,
            agent_id,
            owner_user_id,
            install_id,
            credential_id,
            credential_revision,
            credential_digest,
            session_id,
            server_instance_id,
            agent_version,
            presented_secret,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_planning_bootstrap_v14(
        agent_id: String,
        owner_user_id: String,
        install_id: String,
        credential_id: String,
        credential_revision: u64,
        credential_digest: String,
        session_id: String,
        server_instance_id: String,
        agent_version: String,
        presented_secret: String,
    ) -> Result<Self> {
        Self::new_for_profile(
            NodeEndpointSessionProfile::PlanningSnapshotBootstrapV14,
            agent_id,
            owner_user_id,
            install_id,
            credential_id,
            credential_revision,
            credential_digest,
            session_id,
            server_instance_id,
            agent_version,
            presented_secret,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_for_profile(
        profile: NodeEndpointSessionProfile,
        agent_id: String,
        owner_user_id: String,
        install_id: String,
        credential_id: String,
        credential_revision: u64,
        credential_digest: String,
        session_id: String,
        server_instance_id: String,
        agent_version: String,
        presented_secret: String,
    ) -> Result<Self> {
        if !bounded_identifier(&agent_id, 160)
            || !bounded_identifier(&owner_user_id, 160)
            || !bounded_identifier(&install_id, 512)
            || !bounded_identifier(&credential_id, 160)
            || !safe_positive(credential_revision)
            || !is_sha256(&credential_digest)
            || !bounded_identifier(&session_id, 160)
            || !bounded_identifier(&server_instance_id, 160)
            || !bounded_identifier(&agent_version, 160)
        {
            bail!("NODE_ENDPOINT_SESSION_OPEN_REQUEST_INVALID");
        }
        Ok(Self {
            agent_id,
            owner_user_id,
            install_id,
            credential_id,
            credential_revision,
            credential_digest,
            session_id,
            server_instance_id,
            protocol_version: profile.protocol_version(),
            agent_version,
            capabilities: profile.capabilities(),
            profile,
            presented_secret: PresentedNodeEndpointCredentialSecret::from_endpoint_bearer(
                presented_secret,
            )?,
        })
    }
}
