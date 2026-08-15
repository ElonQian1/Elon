use std::{fs::File, time::Duration};

use anyhow::{bail, Result};
use chrono::{DateTime, SecondsFormat, Utc};

use crate::{
    compute_federation::{
        external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
        external_pool_adapter_linux_supervisor::launch_external_pool_adapter_supervisor_child,
        external_pool_adapter_supervisor_session::external_pool_adapter_task_protocol_conformance_session_roots,
        external_pool_adapter_task_protocol_conformance::{
            derive_task_protocol_conformance_synthetic_subjects,
            server_task_protocol_conformance_fixture_catalog,
            server_task_protocol_conformance_profile_catalog,
            task_protocol_conformance_delivery_inventory_digest,
            task_protocol_conformance_exchange_inventory_digest,
            task_protocol_conformance_session_roots_digest,
            task_protocol_conformance_task_observation_root,
            validate_task_protocol_conformance_run_evidence,
            TaskProtocolConformanceCleanupEvidence, TaskProtocolConformanceRunEvidence,
        },
    },
    store::compute_external_pool_adapter_task_protocol_conformance::entrypoint_capsule::{
        with_external_pool_adapter_entrypoint_capsule, ExternalPoolAdapterEntrypointSource,
        PreparedExternalPoolAdapterEntrypointCapsule,
    },
};
use elon_external_pool_adapter_session_core::{
    prepare_external_pool_adapter_ephemeral_bundle_delivery,
    prepare_external_pool_adapter_supervisor_session, ExternalPoolAdapterTaskProtocolHost,
};

use super::{
    oracle::StatefulTaskProtocolOracle,
    support::{
        capability_observations, exchange_material, load_public_fixtures, run_nonce_digest,
        PublicFixtureBytes,
    },
    TaskProtocolConformanceExecutionInput,
};
use crate::store::compute_external_pool_adapter_task_protocol_conformance::runtime::ExternalPoolAdapterTaskProtocolConformanceRuntime;

const EXCHANGE_TIMEOUT: Duration = Duration::from_millis(15_000);
const CHILD_EXIT_TIMEOUT: Duration = Duration::from_secs(2);
const FIXTURE_DELIVERY_GENERATION: u64 = 1;

struct RetainedEntrypoint<'a>(&'a PreparedExternalPoolAdapterInstallation);

impl ExternalPoolAdapterEntrypointSource for RetainedEntrypoint<'_> {
    fn retained_entrypoint(&self) -> Result<(&File, &str, u64)> {
        self.0.retained_entrypoint()
    }
}

pub(super) fn execute(
    input: TaskProtocolConformanceExecutionInput,
    runtime: &ExternalPoolAdapterTaskProtocolConformanceRuntime,
) -> Result<TaskProtocolConformanceRunEvidence> {
    audit_execution_input(&input)?;
    let profile = server_task_protocol_conformance_profile_catalog()?;
    let fixture_catalog = server_task_protocol_conformance_fixture_catalog()?;
    if input.registry_release.supported_capabilities != profile.profile.required_capabilities {
        bail!("task conformance release lacks the exact frozen six-capability set");
    }
    let subjects = derive_task_protocol_conformance_synthetic_subjects(
        &input.registry_release,
        &profile.profile_digest,
        &fixture_catalog.catalog_digest,
    )?;
    let run_nonce_digest = run_nonce_digest()?;
    let fixtures = load_public_fixtures(&input.prepared_installation, &input.fixture_resources)?;
    let source = RetainedEntrypoint(&input.prepared_installation);
    let run_started = Utc::now();
    let mut output = None;
    with_external_pool_adapter_entrypoint_capsule(&source, |capsule| {
        output = Some(execute_capsule(
            &input,
            runtime,
            capsule,
            &fixtures,
            &profile.profile_digest,
            &fixture_catalog.catalog_digest,
            &subjects.fixture_lane.subject_digest,
            &subjects.fixture_executor.subject_digest,
            &run_nonce_digest,
            run_started,
        )?);
        Ok(())
    })?;
    output.ok_or_else(|| anyhow::anyhow!("task conformance capsule produced no evidence"))
}

#[allow(clippy::too_many_arguments)]
fn execute_capsule(
    input: &TaskProtocolConformanceExecutionInput,
    runtime: &ExternalPoolAdapterTaskProtocolConformanceRuntime,
    capsule: &PreparedExternalPoolAdapterEntrypointCapsule,
    fixtures: &PublicFixtureBytes,
    profile_digest: &str,
    fixture_catalog_digest: &str,
    fixture_lane_digest: &str,
    fixture_executor_digest: &str,
    run_nonce_digest: &str,
    run_started: DateTime<Utc>,
) -> Result<TaskProtocolConformanceRunEvidence> {
    if capsule.entrypoint_sha256() != input.source_capsule_sha256
        || capsule.entrypoint_size_bytes() != input.source_capsule_size_bytes
        || capsule.launch_sha256() != input.launch_image_sha256
        || capsule.launch_size_bytes() != input.launch_image_size_bytes
        || capsule.entrypoint_sha256() == capsule.launch_sha256()
    {
        bail!("task conformance materialized source/launch roots drifted from exact V268 roots");
    }
    let delivery = prepare_external_pool_adapter_ephemeral_bundle_delivery(
        FIXTURE_DELIVERY_GENERATION,
        &fixtures.config,
        &fixtures.credential,
    )?;
    let public_fixture_delivery_root = delivery.bundle_root_hex();
    let session_root_digests = [
        input.supervisor_session_policy_digest.clone(),
        profile_digest.to_owned(),
        run_nonce_digest.to_owned(),
        fixture_catalog_digest.to_owned(),
        input.registry_release.registry_release_digest.clone(),
        input.registry_release.installation_content_digest.clone(),
        input.registry_release.capability_set_digest.clone(),
        input.sandbox_reattestation_receipt_digest.clone(),
        input
            .runtime_compatibility_verification_receipt_digest
            .clone(),
        input.source_capsule_sha256.clone(),
        input.launch_image_sha256.clone(),
        public_fixture_delivery_root.clone(),
        fixture_lane_digest.to_owned(),
        fixture_executor_digest.to_owned(),
    ];
    let session_roots_digest =
        task_protocol_conformance_session_roots_digest(&session_root_digests)?;
    let [supervisor_session_policy_digest, task_protocol_profile_digest, run_nonce_digest, fixture_catalog_digest, registry_release_digest, installation_content_digest, capability_set_digest, sandbox_reattestation_receipt_digest, runtime_compatibility_verification_receipt_digest, source_capsule_sha256, launch_image_sha256, public_fixture_delivery_root_digest, synthetic_fixture_lane_digest, synthetic_fixture_executor_digest] =
        &session_root_digests;
    let roots = external_pool_adapter_task_protocol_conformance_session_roots(
        supervisor_session_policy_digest,
        task_protocol_profile_digest,
        run_nonce_digest,
        fixture_catalog_digest,
        registry_release_digest,
        installation_content_digest,
        capability_set_digest,
        sandbox_reattestation_receipt_digest,
        runtime_compatibility_verification_receipt_digest,
        source_capsule_sha256,
        launch_image_sha256,
        public_fixture_delivery_root_digest,
        synthetic_fixture_lane_digest,
        synthetic_fixture_executor_digest,
    )?;
    let (host, child_bootstrap) = prepare_external_pool_adapter_supervisor_session(roots)?.split();
    let mut child = launch_external_pool_adapter_supervisor_child(
        runtime.cgroup_parent(),
        child_bootstrap,
        capsule,
    )?;
    let mut session = host.authenticate()?;
    let delivery_receipt = delivery.deliver(
        &mut session,
        &public_fixture_delivery_root,
        &fixtures.config,
        &fixtures.credential,
    )?;
    if delivery_receipt.bundle_root_hex() != public_fixture_delivery_root {
        bail!("task conformance fresh public fixture delivery root drifted");
    }
    let mut protocol = ExternalPoolAdapterTaskProtocolHost::new(&mut session);
    let session_transcript_digest = protocol.session_transcript_digest_hex();
    if session_transcript_digest != session_roots_digest {
        bail!("task conformance authenticated session transcript root drifted");
    }
    let mut oracle = StatefulTaskProtocolOracle::new()?;
    let mut exchanges = Vec::with_capacity(8);
    for ordinal in 1..=8 {
        let material = exchange_material(
            ordinal,
            run_nonce_digest,
            fixture_lane_digest,
            fixture_executor_digest,
        )?;
        exchanges.push(oracle.execute_exchange(&mut protocol, material, EXCHANGE_TIMEOUT)?);
    }
    drop(protocol);
    delivery_receipt.shutdown(&mut session)?;
    let exit = child
        .wait(CHILD_EXIT_TIMEOUT)?
        .ok_or_else(|| anyhow::anyhow!("task conformance child did not exit in reap window"))?;
    if exit.exit_code != Some(0) || exit.signal.is_some() {
        bail!("task conformance child failed authenticated shutdown");
    }
    if !child.collect_stderr()?.is_empty() {
        bail!("task conformance child emitted stderr");
    }
    let cleanup = TaskProtocolConformanceCleanupEvidence {
        authenticated_shutdown_completed: true,
        pidfd_reaped: true,
        cgroup_cleaned: true,
        scratch_cleaned: true,
    };
    let delivery_inventory_digest =
        task_protocol_conformance_delivery_inventory_digest(&exchanges)?;
    let exchange_inventory_digest =
        task_protocol_conformance_exchange_inventory_digest(&exchanges)?;
    let capabilities =
        capability_observations(exchanges.as_slice(), profile_digest, fixture_catalog_digest)?;
    let task_observation_root =
        task_protocol_conformance_task_observation_root(&exchanges, &capabilities, &cleanup)?;
    let run_completed = Utc::now();
    let elapsed = run_completed
        .signed_duration_since(run_started.clone())
        .num_milliseconds();
    if elapsed < 0 {
        bail!("task conformance wall clock moved backwards");
    }
    let evidence = TaskProtocolConformanceRunEvidence {
        run_nonce_digest: run_nonce_digest.to_owned(),
        source_capsule_sha256: capsule.entrypoint_sha256().to_owned(),
        source_capsule_size_bytes: capsule.entrypoint_size_bytes(),
        launch_image_sha256: capsule.launch_sha256().to_owned(),
        launch_image_size_bytes: capsule.launch_size_bytes(),
        public_fixture_delivery_root,
        session_roots_digest,
        session_transcript_digest,
        delivery_inventory_digest,
        exchange_inventory_digest,
        task_observation_root,
        run_started_at: canonical_time(run_started),
        run_completed_at: canonical_time(run_completed),
        duration_ms: u64::try_from(elapsed)?,
        exchanges,
        capabilities,
        cleanup,
    };
    validate_task_protocol_conformance_run_evidence(
        &evidence,
        profile_digest,
        fixture_catalog_digest,
    )?;
    Ok(evidence)
}

fn audit_execution_input(input: &TaskProtocolConformanceExecutionInput) -> Result<()> {
    let binding = input.prepared_installation.binding();
    let (_, entrypoint_sha256, entrypoint_size_bytes) =
        input.prepared_installation.retained_entrypoint()?;
    if input.source_capsule_size_bytes == 0
        || input.launch_image_size_bytes == 0
        || input.source_capsule_sha256 == input.launch_image_sha256
        || input.source_capsule_sha256 != input.registry_release.entrypoint_sha256
        || input.source_capsule_size_bytes != input.registry_release.entrypoint_size_bytes
        || entrypoint_sha256 != input.source_capsule_sha256
        || entrypoint_size_bytes != input.source_capsule_size_bytes
        || binding.entrypoint_sha256 != input.source_capsule_sha256
        || binding.entrypoint_size_bytes != input.source_capsule_size_bytes
        || binding.installation_content_digest != input.registry_release.installation_content_digest
        || binding.capability_set_digest != input.registry_release.capability_set_digest
        || binding.adapter_id != input.registry_release.adapter_id
        || binding.adapter_release_version != input.registry_release.release_version
    {
        bail!("task conformance Prepared execution carrier is not exact neutral release content");
    }
    Ok(())
}

fn canonical_time(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Nanos, true)
}
