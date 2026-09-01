use super::{
    metering::{
        InteractiveDesktopCumulativeCounter, InteractiveDesktopMeter,
        InteractiveDesktopMeterSettlementClass, InteractiveDesktopUsageLayer,
        InteractiveDesktopUsageSourceKind, InteractiveDesktopUsageVerificationBinding,
        InteractiveDesktopUsageVerificationDagNode,
        InteractiveDesktopUsageVerificationPolicyBinding,
        InteractiveDesktopUsageVerificationReceipt, InteractiveDesktopUsageVerificationStatus,
        INTERACTIVE_DESKTOP_USAGE_VERIFICATION_DAG_NODE_SCHEMA,
        INTERACTIVE_DESKTOP_USAGE_VERIFICATION_RECEIPT_SCHEMA,
    },
    offer::{InteractiveDesktopConnectivityPolicy, InteractiveDesktopTransportPath},
    test_support::pending_usage_receipt,
};

#[test]
fn metering_is_monotonic_integer_and_layer_specific() {
    let counter = InteractiveDesktopCumulativeCounter {
        meter: InteractiveDesktopMeter::VideoBytes,
        opening_quantity: 10,
        closing_quantity: 42,
    };
    assert_eq!(counter.delivered_quantity(), Some(32));

    let layer = InteractiveDesktopUsageLayer {
        source_kind: InteractiveDesktopUsageSourceKind::TransportObserved,
        source_ref_digest: "transport-source-digest".to_string(),
        sample_sequence: 7,
        previous_sample_digest: Some("observation-digest-6".to_string()),
        counters: vec![counter],
        observation_digest: "observation-digest".to_string(),
        observed_at_ms: 1_500,
    };
    assert!(layer.has_monotonic_unique_counters());

    let regressing = InteractiveDesktopUsageLayer {
        counters: vec![InteractiveDesktopCumulativeCounter {
            meter: InteractiveDesktopMeter::VideoBytes,
            opening_quantity: 43,
            closing_quantity: 42,
        }],
        ..layer.clone()
    };
    assert!(!regressing.has_monotonic_unique_counters());

    let duplicate = InteractiveDesktopUsageLayer {
        counters: vec![
            InteractiveDesktopCumulativeCounter {
                meter: InteractiveDesktopMeter::VideoBytes,
                opening_quantity: 0,
                closing_quantity: 1,
            },
            InteractiveDesktopCumulativeCounter {
                meter: InteractiveDesktopMeter::VideoBytes,
                opening_quantity: 1,
                closing_quantity: 2,
            },
        ],
        ..layer.clone()
    };
    assert!(!duplicate.has_monotonic_unique_counters());

    let next = InteractiveDesktopUsageLayer {
        sample_sequence: 8,
        previous_sample_digest: Some("observation-digest".to_string()),
        counters: vec![InteractiveDesktopCumulativeCounter {
            meter: InteractiveDesktopMeter::VideoBytes,
            opening_quantity: 42,
            closing_quantity: 50,
        }],
        observation_digest: "observation-digest-8".to_string(),
        ..layer.clone()
    };
    assert!(next.continues_after(&layer));
    let reset = InteractiveDesktopUsageLayer {
        counters: vec![InteractiveDesktopCumulativeCounter {
            meter: InteractiveDesktopMeter::VideoBytes,
            opening_quantity: 0,
            closing_quantity: 8,
        }],
        ..next
    };
    assert!(!reset.continues_after(&layer));
    assert!(!InteractiveDesktopMeter::InputEvents.is_compensable_v1());
    assert!(!InteractiveDesktopMeter::VideoFrames.is_compensable_v1());
    assert!(!InteractiveDesktopMeter::TurnEgressBytes.is_compensable_v1());
    assert_eq!(
        InteractiveDesktopMeter::TurnEgressBytes.settlement_class_v1(),
        InteractiveDesktopMeterSettlementClass::PlatformRelayCost,
    );
}

#[test]
fn usage_requires_current_epochs_and_offer_transport_policy() {
    let mut usage = pending_usage_receipt();
    usage.media_epoch_id.clear();
    assert!(!usage.has_valid_layer_boundaries());

    usage = pending_usage_receipt();
    usage.control_epoch_sequence = 0;
    assert!(!usage.has_valid_layer_boundaries());

    usage = pending_usage_receipt();
    usage.binding.offer.connectivity_policy = InteractiveDesktopConnectivityPolicy::RelayOnly;
    assert!(!usage.has_valid_layer_boundaries());
    usage.transport_path = InteractiveDesktopTransportPath::Turn;
    assert!(usage.has_valid_layer_boundaries());
}

#[test]
fn usage_receipts_form_one_continuous_non_overlapping_chain() {
    let first = pending_usage_receipt();
    assert!(first.has_valid_layer_boundaries());

    let mut next = first.clone();
    next.usage_receipt_id = "usage-2".to_string();
    next.usage_receipt_digest = "usage-digest-2".to_string();
    next.usage_sequence = 2;
    next.previous_usage_receipt_digest = Some(first.usage_receipt_digest.clone());
    next.session_revision = 6;
    next.session_digest = "session-digest-6".to_string();
    next.interval_started_at_ms = first.interval_ended_at_ms;
    next.interval_ended_at_ms = 2_000;
    advance_usage_layer(&mut next.declared, 1_000);
    advance_usage_layer(&mut next.transport_observed, 980);
    advance_usage_layer(&mut next.consumer_observed, 960);
    assert!(next.has_valid_layer_boundaries());
    assert!(next.continues_after(&first));

    let mut overlap = next.clone();
    overlap.interval_started_at_ms = first.interval_ended_at_ms - 1;
    assert!(!overlap.continues_after(&first));

    let mut reset = next;
    reset.declared.counters[0].opening_quantity = 0;
    assert!(!reset.continues_after(&first));
}

#[test]
fn accepted_verification_requires_non_empty_usage_digests() {
    let mut receipt = InteractiveDesktopUsageVerificationReceipt {
        schema: INTERACTIVE_DESKTOP_USAGE_VERIFICATION_RECEIPT_SCHEMA.to_string(),
        service_class: super::INTERACTIVE_DESKTOP_SERVICE_CLASS.to_string(),
        verification_receipt_id: "verification-1".to_string(),
        verification_receipt_digest: "verification-digest".to_string(),
        usage_receipt_id: "usage-1".to_string(),
        usage_receipt_digest: "usage-digest-1".to_string(),
        session_id: "session-1".to_string(),
        binding_digest: "binding-digest".to_string(),
        policy_id: "conservative-min-v1".to_string(),
        policy_version: 1,
        status: InteractiveDesktopUsageVerificationStatus::Accepted,
        reason_codes: Vec::new(),
        verified_usage_digest: Some(String::new()),
        compensable_usage_digest: Some(String::new()),
        decided_at_ms: 2_000,
    };
    assert!(!receipt.has_consistent_decision());
    receipt.verified_usage_digest = Some("verified-digest".to_string());
    receipt.compensable_usage_digest = Some("compensable-digest".to_string());
    assert!(receipt.has_consistent_decision());
}

#[test]
fn verification_is_a_one_way_cross_validated_dag_node() {
    let usage = pending_usage_receipt();
    let mut verified = usage.consumer_observed.clone();
    verified.source_kind = InteractiveDesktopUsageSourceKind::Verified;
    verified.source_ref_digest = "verification-source-digest".to_string();
    verified.observation_digest = "verified-usage-digest".to_string();
    let mut compensable = verified.clone();
    compensable.source_kind = InteractiveDesktopUsageSourceKind::Compensable;
    compensable.source_ref_digest = "compensation-source-digest".to_string();
    compensable.observation_digest = "compensable-usage-digest".to_string();

    let expected = InteractiveDesktopUsageVerificationBinding {
        verification_receipt_id: "verification-1".to_string(),
        verification_receipt_digest: "verification-digest".to_string(),
        status: InteractiveDesktopUsageVerificationStatus::Accepted,
    };
    let node = InteractiveDesktopUsageVerificationDagNode {
        schema: INTERACTIVE_DESKTOP_USAGE_VERIFICATION_DAG_NODE_SCHEMA.to_string(),
        verification: InteractiveDesktopUsageVerificationReceipt {
            schema: INTERACTIVE_DESKTOP_USAGE_VERIFICATION_RECEIPT_SCHEMA.to_string(),
            service_class: super::INTERACTIVE_DESKTOP_SERVICE_CLASS.to_string(),
            verification_receipt_id: expected.verification_receipt_id.clone(),
            verification_receipt_digest: expected.verification_receipt_digest.clone(),
            usage_receipt_id: usage.usage_receipt_id.clone(),
            usage_receipt_digest: usage.usage_receipt_digest.clone(),
            session_id: usage.session_id.clone(),
            binding_digest: usage.binding.binding_digest.clone(),
            policy_id: "conservative-min-v1".to_string(),
            policy_version: 1,
            status: InteractiveDesktopUsageVerificationStatus::Accepted,
            reason_codes: Vec::new(),
            verified_usage_digest: Some(verified.observation_digest.clone()),
            compensable_usage_digest: Some(compensable.observation_digest.clone()),
            decided_at_ms: 2_000,
        },
        policy: InteractiveDesktopUsageVerificationPolicyBinding {
            policy_id: "conservative-min-v1".to_string(),
            policy_version: 1,
            policy_digest: "verification-policy-digest".to_string(),
        },
        verified: Some(verified),
        compensable: Some(compensable),
    };

    assert!(node.cross_validates_raw_usage(&usage, &expected, "verification-policy-digest",));
    assert!(!node.cross_validates_raw_usage(&usage, &expected, "other-policy-digest"));

    let mut wrong_usage = usage;
    wrong_usage.binding.binding_digest = "other-binding-digest".to_string();
    assert!(!node.cross_validates_raw_usage(&wrong_usage, &expected, "verification-policy-digest",));
}

fn advance_usage_layer(layer: &mut InteractiveDesktopUsageLayer, closing_quantity: u64) {
    let previous_digest = layer.observation_digest.clone();
    layer.sample_sequence += 1;
    layer.previous_sample_digest = Some(previous_digest);
    layer.counters[0].opening_quantity = layer.counters[0].closing_quantity;
    layer.counters[0].closing_quantity = closing_quantity;
    layer.observation_digest = format!("{}-next", layer.source_ref_digest);
}
