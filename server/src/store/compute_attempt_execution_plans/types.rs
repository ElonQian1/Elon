use serde::Serialize;

use crate::{
    compute_federation::{
        capacity::ComputeCapacityClaim,
        execution_plan::{
            ComputeArtifactAccessEnvelope, ComputeAttemptExecutionPlanEnvelope,
            ComputeAttemptExecutionPlanSealEnvelope, ComputeExecutionCapabilityEnvelope,
        },
    },
    store::{
        compute_broker_reservation::BrokerReserveBinding, ComputeJobRegistrationReceipt,
        ComputeOfferRegistrationReceipt, ComputeProviderRegistrationReceipt,
        ComputeReservationRegistrationReceipt,
    },
};

#[derive(Debug, Serialize)]
pub(crate) struct ComputeAttemptExecutionPlanReceipt {
    plan: ComputeAttemptExecutionPlanEnvelope,
    seal: ComputeAttemptExecutionPlanSealEnvelope,
    replayed: bool,
}

impl ComputeAttemptExecutionPlanReceipt {
    pub(super) fn new(
        plan: ComputeAttemptExecutionPlanEnvelope,
        seal: ComputeAttemptExecutionPlanSealEnvelope,
        replayed: bool,
    ) -> Self {
        Self {
            plan,
            seal,
            replayed,
        }
    }

    pub(crate) fn plan(&self) -> &ComputeAttemptExecutionPlanEnvelope {
        &self.plan
    }

    pub(crate) fn seal(&self) -> &ComputeAttemptExecutionPlanSealEnvelope {
        &self.seal
    }

    pub(crate) fn replayed(&self) -> bool {
        self.replayed
    }
}

pub(super) struct PreparedCapability {
    pub envelope: ComputeExecutionCapabilityEnvelope,
    pub canonical_json: String,
    pub digest: String,
}

pub(super) struct PreparedArtifactAccess {
    pub envelope: ComputeArtifactAccessEnvelope,
    pub canonical_json: String,
    pub digest: String,
}

pub(super) struct PreparedInputs {
    pub capability: PreparedCapability,
    pub accesses: Vec<PreparedArtifactAccess>,
}

pub(super) struct CurrentExecutionSources {
    pub historical_provider: ComputeProviderRegistrationReceipt,
    pub historical_offer: ComputeOfferRegistrationReceipt,
    pub job: ComputeJobRegistrationReceipt,
    pub reservation: ComputeReservationRegistrationReceipt,
    pub claim: ComputeCapacityClaim,
    pub broker: BrokerReserveBinding,
    pub broker_request_digest: String,
    pub budget_expires_at: Option<String>,
}

pub(super) struct PreparedPlan {
    pub plan: ComputeAttemptExecutionPlanEnvelope,
    pub plan_json: String,
    pub plan_digest: String,
    pub access_set_digest: String,
    pub resource_grant_json: String,
    pub resource_grant_digest: String,
    pub seal: ComputeAttemptExecutionPlanSealEnvelope,
    pub seal_json: String,
    pub seal_digest: String,
}
