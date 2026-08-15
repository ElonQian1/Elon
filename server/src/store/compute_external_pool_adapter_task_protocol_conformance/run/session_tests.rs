use std::{thread, time::Duration};

use anyhow::Result;
use elon_external_pool_adapter_session_core::{
    prepare_external_pool_adapter_ephemeral_bundle_delivery,
    prepare_external_pool_adapter_supervisor_session,
    receive_external_pool_adapter_ephemeral_bundle_from_begin, ExternalPoolAdapterTaskProtocolHost,
};

use crate::compute_federation::{
    external_pool_adapter_supervisor_session::external_pool_adapter_task_protocol_conformance_session_roots,
    external_pool_adapter_supervisor_session_policy_companion::server_supervisor_session_policy_catalog,
    external_pool_adapter_task_protocol_conformance::{
        server_task_protocol_conformance_fixture_catalog,
        server_task_protocol_conformance_profile_catalog,
        task_protocol_conformance_session_roots_digest,
    },
};

use super::{
    oracle::StatefulTaskProtocolOracle,
    support::{capability_observations, exchange_material, run_nonce_digest},
};

#[path = "../../../external_pool_adapter_session_fixture/task_protocol_conformance.rs"]
mod child_fixture;

const CONFIG: &[u8] = br#"{"mode":"v272-authenticated-session-wire"}"#;
const CREDENTIAL: &[u8] = b"v272-test-credential-never-production";
const EXCHANGE_TIMEOUT: Duration = Duration::from_millis(15_000);

#[test]
fn v272_linux_authenticated_session_wire_executes_exact_eight_exchanges() -> Result<()> {
    let profile = server_task_protocol_conformance_profile_catalog()?;
    let fixture_catalog = server_task_protocol_conformance_fixture_catalog()?;
    let (_, supervisor_session_policy_digest) = server_supervisor_session_policy_catalog()?;
    let run_nonce_digest = run_nonce_digest()?;
    let delivery = prepare_external_pool_adapter_ephemeral_bundle_delivery(1, CONFIG, CREDENTIAL)?;
    let public_fixture_delivery_root = delivery.bundle_root_hex();
    let registry_release_digest = digest(0x51);
    let installation_content_digest = digest(0x52);
    let capability_set_digest = digest(0x53);
    let sandbox_reattestation_receipt_digest = digest(0x54);
    let runtime_compatibility_verification_receipt_digest = digest(0x55);
    let source_capsule_sha256 = digest(0x56);
    let launch_image_sha256 = digest(0x57);
    let fixture_lane_digest = digest(0x58);
    let fixture_executor_digest = digest(0x59);
    let session_root_digests = vec![
        supervisor_session_policy_digest.clone(),
        profile.profile_digest.clone(),
        run_nonce_digest.clone(),
        fixture_catalog.catalog_digest.clone(),
        registry_release_digest.clone(),
        installation_content_digest.clone(),
        capability_set_digest.clone(),
        sandbox_reattestation_receipt_digest.clone(),
        runtime_compatibility_verification_receipt_digest.clone(),
        source_capsule_sha256.clone(),
        launch_image_sha256.clone(),
        public_fixture_delivery_root.clone(),
        fixture_lane_digest.clone(),
        fixture_executor_digest.clone(),
    ];
    let expected_session_transcript =
        task_protocol_conformance_session_roots_digest(&session_root_digests)?;
    let roots = external_pool_adapter_task_protocol_conformance_session_roots(
        &supervisor_session_policy_digest,
        &profile.profile_digest,
        &run_nonce_digest,
        &fixture_catalog.catalog_digest,
        &registry_release_digest,
        &installation_content_digest,
        &capability_set_digest,
        &sandbox_reattestation_receipt_digest,
        &runtime_compatibility_verification_receipt_digest,
        &source_capsule_sha256,
        &launch_image_sha256,
        &public_fixture_delivery_root,
        &fixture_lane_digest,
        &fixture_executor_digest,
    )?;
    let (host_bootstrap, child_bootstrap) =
        prepare_external_pool_adapter_supervisor_session(roots.clone())?.split();
    let child_delivery_root = public_fixture_delivery_root.clone();
    let child = thread::spawn(move || -> Result<()> {
        let mut session = child_bootstrap.authenticate(roots)?;
        let first = session.receive()?;
        let delivered = receive_external_pool_adapter_ephemeral_bundle_from_begin(
            &mut session,
            &child_delivery_root,
            first,
        )?;
        child_fixture::execute(&mut session, delivered, CONFIG, CREDENTIAL)
    });

    let mut session = host_bootstrap.authenticate()?;
    let delivery_receipt = delivery.deliver(
        &mut session,
        &public_fixture_delivery_root,
        CONFIG,
        CREDENTIAL,
    )?;
    let mut protocol = ExternalPoolAdapterTaskProtocolHost::new(&mut session);
    assert_eq!(
        protocol.session_transcript_digest_hex(),
        expected_session_transcript
    );
    let mut oracle = StatefulTaskProtocolOracle::new()?;
    let mut exchanges = Vec::with_capacity(8);
    for ordinal in 1..=8 {
        let material = exchange_material(
            ordinal,
            &run_nonce_digest,
            &fixture_lane_digest,
            &fixture_executor_digest,
        )?;
        exchanges.push(oracle.execute_exchange(&mut protocol, material, EXCHANGE_TIMEOUT)?);
    }
    drop(protocol);
    delivery_receipt.shutdown(&mut session)?;
    child
        .join()
        .map_err(|_| anyhow::anyhow!("V272 authenticated child thread panicked"))??;

    assert_eq!(exchanges.len(), 8);
    assert!(exchanges
        .iter()
        .enumerate()
        .all(|(index, exchange)| exchange.exchange_ordinal == (index + 1) as u64));
    assert!(exchanges.iter().all(|exchange| {
        exchange.upstream_response_bytes == 2_048
            && exchange.exchange_root.len() == 64
            && exchange.semantic_observation_sha256.len() == 64
    }));
    assert_eq!(exchanges[1].oracle_start_count_after, 1);
    assert_eq!(exchanges[2].oracle_start_count_after, 1);
    assert_eq!(
        exchanges[3].commit_uncertainty_state_after,
        "resolved_by_reconcile"
    );
    assert_eq!(exchanges[4].event_count, 2);
    assert_eq!(
        exchanges[4].event_replay_classification.as_deref(),
        Some("exact_duplicate_batch_replay")
    );
    assert!(exchanges[6].no_commit_tombstone_digest.is_none());
    assert!(exchanges[7].no_commit_tombstone_digest.is_some());
    assert_eq!(exchanges[7].oracle_start_count_after, 0);
    assert_eq!(exchanges[7].oracle_event_count_after, 0);
    let capabilities = capability_observations(
        &exchanges,
        &profile.profile_digest,
        &fixture_catalog.catalog_digest,
    )?;
    assert_eq!(capabilities.len(), 6);
    assert!(capabilities
        .iter()
        .all(|capability| capability.status == "passed_server_run"));
    Ok(())
}

fn digest(byte: u8) -> String {
    format!("{byte:02x}").repeat(32)
}
