use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{
    offer::{InteractiveDesktopConnectivityPolicy, InteractiveDesktopTransportPath},
    session::InteractiveDesktopFederationBinding,
    INTERACTIVE_DESKTOP_SERVICE_CLASS,
};

pub(crate) const INTERACTIVE_DESKTOP_USAGE_RECEIPT_SCHEMA: &str =
    "compute_federation.interactive_desktop.usage_receipt.v1";
pub(crate) const INTERACTIVE_DESKTOP_USAGE_VERIFICATION_RECEIPT_SCHEMA: &str =
    "compute_federation.interactive_desktop.usage_verification_receipt.v1";
pub(crate) const INTERACTIVE_DESKTOP_USAGE_RECEIPT_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-INTERACTIVE-DESKTOP-USAGE-RECEIPT-V1";
pub(crate) const INTERACTIVE_DESKTOP_USAGE_VERIFICATION_RECEIPT_DIGEST_DOMAIN: &str =
    "ELON-COMPUTE-INTERACTIVE-DESKTOP-USAGE-VERIFICATION-RECEIPT-V1";
pub(crate) const INTERACTIVE_DESKTOP_USAGE_VERIFICATION_DAG_NODE_SCHEMA: &str =
    "compute_federation.interactive_desktop.usage_verification_dag_node.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopMeter {
    MediaActiveMilliseconds,
    VideoBytes,
    AudioBytes,
    VideoFrames,
    AudioPackets,
    InputEvents,
    TurnEgressBytes,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopMeterSettlementClass {
    ProviderCompensable,
    PlatformRelayCost,
    EvidenceOnly,
}

impl InteractiveDesktopMeter {
    pub(crate) const fn settlement_class_v1(self) -> InteractiveDesktopMeterSettlementClass {
        match self {
            Self::MediaActiveMilliseconds | Self::VideoBytes | Self::AudioBytes => {
                InteractiveDesktopMeterSettlementClass::ProviderCompensable
            }
            Self::TurnEgressBytes => InteractiveDesktopMeterSettlementClass::PlatformRelayCost,
            Self::VideoFrames | Self::AudioPackets | Self::InputEvents => {
                InteractiveDesktopMeterSettlementClass::EvidenceOnly
            }
        }
    }

    /// Compatibility name for the provider-compensable v1 meter set.
    pub(crate) fn is_compensable_v1(self) -> bool {
        self.settlement_class_v1() == InteractiveDesktopMeterSettlementClass::ProviderCompensable
    }

    pub(crate) fn is_platform_relay_cost_v1(self) -> bool {
        self.settlement_class_v1() == InteractiveDesktopMeterSettlementClass::PlatformRelayCost
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopUsageSourceKind {
    ProviderDeclared,
    TransportObserved,
    ConsumerObserved,
    Verified,
    Compensable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopCumulativeCounter {
    pub meter: InteractiveDesktopMeter,
    pub opening_quantity: u64,
    pub closing_quantity: u64,
}

impl InteractiveDesktopCumulativeCounter {
    pub(crate) fn delivered_quantity(&self) -> Option<u64> {
        self.closing_quantity.checked_sub(self.opening_quantity)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopUsageLayer {
    pub source_kind: InteractiveDesktopUsageSourceKind,
    pub source_ref_digest: String,
    pub sample_sequence: u64,
    pub previous_sample_digest: Option<String>,
    pub counters: Vec<InteractiveDesktopCumulativeCounter>,
    pub observation_digest: String,
    pub observed_at_ms: i64,
}

impl InteractiveDesktopUsageLayer {
    pub(crate) fn has_monotonic_unique_counters(&self) -> bool {
        let mut meters = BTreeSet::new();
        self.sample_sequence > 0
            && (self.sample_sequence == 1) == self.previous_sample_digest.is_none()
            && !self.source_ref_digest.is_empty()
            && !self.observation_digest.is_empty()
            && !self.counters.is_empty()
            && self.counters.iter().all(|counter| {
                counter.delivered_quantity().is_some() && meters.insert(counter.meter)
            })
    }

    pub(crate) fn continues_after(&self, previous: &Self) -> bool {
        self.source_kind == previous.source_kind
            && self.source_ref_digest == previous.source_ref_digest
            && previous.sample_sequence.checked_add(1) == Some(self.sample_sequence)
            && self.previous_sample_digest.as_deref() == Some(previous.observation_digest.as_str())
            && self.counters.len() == previous.counters.len()
            && self.counters.iter().all(|counter| {
                previous.counters.iter().any(|prior| {
                    prior.meter == counter.meter
                        && counter.opening_quantity == prior.closing_quantity
                })
            })
    }

    fn is_bounded_by(&self, parent: &Self) -> bool {
        self.counters.iter().all(|counter| {
            parent.counters.iter().any(|upper| {
                upper.meter == counter.meter
                    && counter.delivered_quantity().is_some_and(|quantity| {
                        upper
                            .delivered_quantity()
                            .is_some_and(|upper_quantity| quantity <= upper_quantity)
                    })
            })
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InteractiveDesktopUsageVerificationStatus {
    Pending,
    Accepted,
    Rejected,
    Disputed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopUsageVerificationBinding {
    pub verification_receipt_id: String,
    pub verification_receipt_digest: String,
    pub status: InteractiveDesktopUsageVerificationStatus,
}

impl InteractiveDesktopUsageVerificationBinding {
    fn has_complete_reference(&self) -> bool {
        !self.verification_receipt_id.is_empty() && !self.verification_receipt_digest.is_empty()
    }
}

/// Aggregate-only session usage. It has no signaling, media, audio, or input payload field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopUsageReceipt {
    pub schema: String,
    pub service_class: String,
    pub usage_receipt_id: String,
    pub usage_receipt_digest: String,
    pub usage_sequence: u64,
    pub previous_usage_receipt_digest: Option<String>,
    pub session_id: String,
    pub session_root_digest: String,
    pub session_revision: i64,
    pub session_digest: String,
    pub binding: InteractiveDesktopFederationBinding,
    pub host_lease_id: String,
    pub fencing_generation: u64,
    pub viewer_grant_id: String,
    pub viewer_grant_generation: u64,
    pub selected_surface_digest: String,
    pub media_epoch_id: String,
    pub media_epoch_sequence: u64,
    pub control_epoch_id: String,
    pub control_epoch_sequence: u64,
    pub transport_path: InteractiveDesktopTransportPath,
    pub interval_started_at_ms: i64,
    pub interval_ended_at_ms: i64,
    pub declared: InteractiveDesktopUsageLayer,
    pub transport_observed: InteractiveDesktopUsageLayer,
    pub consumer_observed: InteractiveDesktopUsageLayer,
    pub created_at_ms: i64,
}

impl InteractiveDesktopUsageReceipt {
    pub(crate) fn has_valid_layer_boundaries(&self) -> bool {
        self.schema == INTERACTIVE_DESKTOP_USAGE_RECEIPT_SCHEMA
            && self.service_class == INTERACTIVE_DESKTOP_SERVICE_CLASS
            && !self.usage_receipt_id.is_empty()
            && !self.usage_receipt_digest.is_empty()
            && !self.session_id.is_empty()
            && !self.session_root_digest.is_empty()
            && self.session_revision > 0
            && !self.session_digest.is_empty()
            && self.binding.has_complete_reference()
            && !self.host_lease_id.is_empty()
            && self.fencing_generation > 0
            && !self.viewer_grant_id.is_empty()
            && self.viewer_grant_generation > 0
            && !self.selected_surface_digest.is_empty()
            && !self.media_epoch_id.is_empty()
            && self.media_epoch_sequence > 0
            && !self.control_epoch_id.is_empty()
            && self.control_epoch_sequence > 0
            && self.usage_sequence > 0
            && (self.usage_sequence == 1) == self.previous_usage_receipt_digest.is_none()
            && (self.usage_sequence != 1
                || (self.declared.sample_sequence == 1
                    && self.transport_observed.sample_sequence == 1
                    && self.consumer_observed.sample_sequence == 1))
            && self
                .previous_usage_receipt_digest
                .as_ref()
                .is_none_or(|digest| !digest.is_empty())
            && (self.binding.offer.connectivity_policy
                != InteractiveDesktopConnectivityPolicy::RelayOnly
                || self.transport_path == InteractiveDesktopTransportPath::Turn)
            && (self.transport_path == InteractiveDesktopTransportPath::Turn
                || !usage_layers(
                    &self.declared,
                    &self.transport_observed,
                    &self.consumer_observed,
                )
                .any(|layer| {
                    layer
                        .counters
                        .iter()
                        .any(|counter| counter.meter.is_platform_relay_cost_v1())
                }))
            && self.interval_ended_at_ms >= self.interval_started_at_ms
            && self.declared.source_kind == InteractiveDesktopUsageSourceKind::ProviderDeclared
            && self.transport_observed.source_kind
                == InteractiveDesktopUsageSourceKind::TransportObserved
            && self.consumer_observed.source_kind
                == InteractiveDesktopUsageSourceKind::ConsumerObserved
            && self.declared.has_monotonic_unique_counters()
            && self.transport_observed.has_monotonic_unique_counters()
            && self.consumer_observed.has_monotonic_unique_counters()
    }

    pub(crate) fn continues_after(&self, previous: &Self) -> bool {
        self.has_valid_layer_boundaries()
            && previous.has_valid_layer_boundaries()
            && self.session_id == previous.session_id
            && self.session_root_digest == previous.session_root_digest
            && self.session_revision >= previous.session_revision
            && (self.session_revision != previous.session_revision
                || self.session_digest == previous.session_digest)
            && self.binding.binding_digest == previous.binding.binding_digest
            && previous.usage_sequence.checked_add(1) == Some(self.usage_sequence)
            && self.previous_usage_receipt_digest.as_deref()
                == Some(previous.usage_receipt_digest.as_str())
            && previous.interval_ended_at_ms <= self.interval_started_at_ms
            && self.declared.continues_after(&previous.declared)
            && self
                .transport_observed
                .continues_after(&previous.transport_observed)
            && self
                .consumer_observed
                .continues_after(&previous.consumer_observed)
    }
}

fn usage_layers<'a>(
    declared: &'a InteractiveDesktopUsageLayer,
    transport_observed: &'a InteractiveDesktopUsageLayer,
    consumer_observed: &'a InteractiveDesktopUsageLayer,
) -> impl Iterator<Item = &'a InteractiveDesktopUsageLayer> {
    [declared, transport_observed, consumer_observed].into_iter()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopUsageVerificationReceipt {
    pub schema: String,
    pub service_class: String,
    pub verification_receipt_id: String,
    pub verification_receipt_digest: String,
    pub usage_receipt_id: String,
    pub usage_receipt_digest: String,
    pub session_id: String,
    pub binding_digest: String,
    pub policy_id: String,
    pub policy_version: i64,
    pub status: InteractiveDesktopUsageVerificationStatus,
    pub reason_codes: Vec<String>,
    pub verified_usage_digest: Option<String>,
    pub compensable_usage_digest: Option<String>,
    pub decided_at_ms: i64,
}

impl InteractiveDesktopUsageVerificationReceipt {
    pub(crate) fn has_consistent_decision(&self) -> bool {
        self.schema == INTERACTIVE_DESKTOP_USAGE_VERIFICATION_RECEIPT_SCHEMA
            && self.service_class == INTERACTIVE_DESKTOP_SERVICE_CLASS
            && !self.verification_receipt_id.is_empty()
            && !self.verification_receipt_digest.is_empty()
            && !self.usage_receipt_id.is_empty()
            && !self.usage_receipt_digest.is_empty()
            && !self.session_id.is_empty()
            && !self.binding_digest.is_empty()
            && !self.policy_id.is_empty()
            && self.policy_version > 0
            && has_bounded_reason_codes(&self.reason_codes)
            && (!matches!(
                self.status,
                InteractiveDesktopUsageVerificationStatus::Rejected
                    | InteractiveDesktopUsageVerificationStatus::Disputed
            ) || !self.reason_codes.is_empty())
            && match self.status {
                InteractiveDesktopUsageVerificationStatus::Accepted => {
                    self.verified_usage_digest
                        .as_ref()
                        .is_some_and(|digest| !digest.is_empty())
                        && self
                            .compensable_usage_digest
                            .as_ref()
                            .is_some_and(|digest| !digest.is_empty())
                }
                InteractiveDesktopUsageVerificationStatus::Pending
                | InteractiveDesktopUsageVerificationStatus::Rejected
                | InteractiveDesktopUsageVerificationStatus::Disputed => {
                    self.verified_usage_digest.is_none() && self.compensable_usage_digest.is_none()
                }
            }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopUsageVerificationPolicyBinding {
    pub policy_id: String,
    pub policy_version: i64,
    pub policy_digest: String,
}

impl InteractiveDesktopUsageVerificationPolicyBinding {
    fn has_complete_reference(&self) -> bool {
        !self.policy_id.is_empty() && self.policy_version > 0 && !self.policy_digest.is_empty()
    }
}

/// One-way verification node: raw Usage is immutable and never references this node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct InteractiveDesktopUsageVerificationDagNode {
    pub schema: String,
    pub verification: InteractiveDesktopUsageVerificationReceipt,
    pub policy: InteractiveDesktopUsageVerificationPolicyBinding,
    pub verified: Option<InteractiveDesktopUsageLayer>,
    pub compensable: Option<InteractiveDesktopUsageLayer>,
}

impl InteractiveDesktopUsageVerificationDagNode {
    pub(crate) fn cross_validates_raw_usage(
        &self,
        usage: &InteractiveDesktopUsageReceipt,
        expected_verification: &InteractiveDesktopUsageVerificationBinding,
        expected_policy_digest: &str,
    ) -> bool {
        let verification_shape_is_valid = match self.verification.status {
            InteractiveDesktopUsageVerificationStatus::Pending
            | InteractiveDesktopUsageVerificationStatus::Rejected
            | InteractiveDesktopUsageVerificationStatus::Disputed => {
                self.verified.is_none() && self.compensable.is_none()
            }
            InteractiveDesktopUsageVerificationStatus::Accepted => {
                self.verified.as_ref().is_some_and(|verified| {
                    verified.source_kind == InteractiveDesktopUsageSourceKind::Verified
                        && verified.has_monotonic_unique_counters()
                        && verified.is_bounded_by(&usage.declared)
                        && verified.is_bounded_by(&usage.transport_observed)
                        && verified.is_bounded_by(&usage.consumer_observed)
                        && self.verification.verified_usage_digest.as_deref()
                            == Some(verified.observation_digest.as_str())
                }) && self.compensable.as_ref().is_some_and(|compensable| {
                    compensable.source_kind == InteractiveDesktopUsageSourceKind::Compensable
                        && compensable.has_monotonic_unique_counters()
                        && compensable
                            .counters
                            .iter()
                            .all(|counter| counter.meter.is_compensable_v1())
                        && self
                            .verified
                            .as_ref()
                            .is_some_and(|verified| compensable.is_bounded_by(verified))
                        && self.verification.compensable_usage_digest.as_deref()
                            == Some(compensable.observation_digest.as_str())
                })
            }
        };

        self.schema == INTERACTIVE_DESKTOP_USAGE_VERIFICATION_DAG_NODE_SCHEMA
            && usage.has_valid_layer_boundaries()
            && self.verification.has_consistent_decision()
            && expected_verification.has_complete_reference()
            && self.verification.verification_receipt_id
                == expected_verification.verification_receipt_id
            && self.verification.verification_receipt_digest
                == expected_verification.verification_receipt_digest
            && self.verification.status == expected_verification.status
            && self.policy.has_complete_reference()
            && !expected_policy_digest.is_empty()
            && self.policy.policy_digest == expected_policy_digest
            && self.verification.policy_id == self.policy.policy_id
            && self.verification.policy_version == self.policy.policy_version
            && self.verification.usage_receipt_id == usage.usage_receipt_id
            && self.verification.usage_receipt_digest == usage.usage_receipt_digest
            && self.verification.session_id == usage.session_id
            && self.verification.binding_digest == usage.binding.binding_digest
            && verification_shape_is_valid
    }
}

fn has_bounded_reason_codes(reason_codes: &[String]) -> bool {
    reason_codes.len() <= 32
        && reason_codes.iter().all(|code| {
            !code.is_empty()
                && code.len() <= 64
                && code
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        })
}
