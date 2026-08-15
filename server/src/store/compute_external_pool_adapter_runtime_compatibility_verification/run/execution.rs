use std::{
    fs::File,
    time::{Duration, Instant},
};

use anyhow::{bail, Result};

use crate::compute_federation::{
    external_pool_adapter_installation::PreparedExternalPoolAdapterInstallation,
    external_pool_adapter_linux_supervisor::{
        launch_external_pool_adapter_supervisor_child, ExternalPoolAdapterSupervisorCgroupParent,
    },
    external_pool_adapter_runtime_compatibility_verification::*,
    external_pool_adapter_supervisor_session::{
        external_pool_adapter_runtime_compatibility_session_roots,
        prepare_external_pool_adapter_ephemeral_bundle_delivery,
        prepare_external_pool_adapter_supervisor_session,
    },
    external_pool_adapter_supervisor_session_policy_companion::server_supervisor_session_policy_catalog,
};
use elon_external_pool_adapter_session_core::receive_external_pool_adapter_no_work_probe_request;

use super::support::RuntimeCompatibilityFixtureBytes;
use crate::store::compute_external_pool_adapter_runtime_compatibility_verification::entrypoint_capsule::{
    with_external_pool_adapter_entrypoint_capsule, ExternalPoolAdapterEntrypointSource,
    PreparedExternalPoolAdapterEntrypointCapsule,
};

const CHILD_EXIT_TIMEOUT: Duration = Duration::from_secs(2);

pub(super) struct RuntimeCompatibilityExecutionEvidence {
    pub(super) source_capsule_sha256: String,
    pub(super) source_capsule_size_bytes: u64,
    pub(super) launch_image_sha256: String,
    pub(super) launch_image_size_bytes: u64,
    pub(super) public_fixture_delivery_root: String,
    pub(super) no_work: ExternalPoolAdapterRuntimeCompatibilityNoWorkEvidence,
    pub(super) duration_ms: u64,
}

struct RetainedEntrypoint<'a>(&'a PreparedExternalPoolAdapterInstallation);

impl ExternalPoolAdapterEntrypointSource for RetainedEntrypoint<'_> {
    fn retained_entrypoint(&self) -> Result<(&File, &str, u64)> {
        self.0.retained_entrypoint()
    }
}

pub(super) fn execute(
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
    prepared: &PreparedExternalPoolAdapterInstallation,
    fixtures: &RuntimeCompatibilityFixtureBytes,
    cgroup_parent: &ExternalPoolAdapterSupervisorCgroupParent,
) -> Result<RuntimeCompatibilityExecutionEvidence> {
    let started = Instant::now();
    let source = RetainedEntrypoint(prepared);
    let mut output = None;
    with_external_pool_adapter_entrypoint_capsule(&source, |capsule| {
        output = Some(execute_capsule(
            challenge,
            capsule,
            fixtures,
            cgroup_parent,
            started,
        )?);
        Ok(())
    })?;
    output.ok_or_else(|| anyhow::anyhow!("V268 capsule execution produced no evidence"))
}

fn execute_capsule(
    challenge: &ExternalPoolAdapterRuntimeCompatibilityChallengeReceipt,
    capsule: &PreparedExternalPoolAdapterEntrypointCapsule,
    fixtures: &RuntimeCompatibilityFixtureBytes,
    cgroup_parent: &ExternalPoolAdapterSupervisorCgroupParent,
    started: Instant,
) -> Result<RuntimeCompatibilityExecutionEvidence> {
    let selected = &challenge.challenge;
    if capsule.entrypoint_sha256() != selected.entrypoint_sha256
        || capsule.entrypoint_size_bytes() != selected.entrypoint_size_bytes
        || capsule.policy_digest() != selected.source_capsule_policy.policy_digest
        || capsule.launch_sha256() == capsule.entrypoint_sha256()
        || capsule.launch_size_bytes() == 0
    {
        bail!("V268 materialized source/launch capsule roots are not exact");
    }
    let delivery = prepare_external_pool_adapter_ephemeral_bundle_delivery(
        selected.sequence,
        &fixtures.config,
        &fixtures.credential,
    )?;
    let delivery_root = delivery.bundle_root_hex();
    let roots = external_pool_adapter_runtime_compatibility_session_roots(
        &selected.supervisor_session_policy.policy_digest,
        &selected.profile_digest,
        &challenge.challenge_digest,
        &selected.runner_policy.policy_digest,
        &selected.fixture_catalog.policy_digest,
        &selected.sandbox_verifier_key_record_digest,
        &selected.registry_release.registry_release_digest,
        &selected
            .registry_release
            .release
            .installation_content_digest,
        capsule.entrypoint_sha256(),
        capsule.launch_sha256(),
        &delivery_root,
    )?;
    let prepared_session = prepare_external_pool_adapter_supervisor_session(roots)?;
    let (host, child_bootstrap) = prepared_session.split();
    let mut child =
        launch_external_pool_adapter_supervisor_child(cgroup_parent, child_bootstrap, capsule)?;
    let mut session = host.authenticate()?;
    let delivery_receipt = delivery.deliver(
        &mut session,
        &delivery_root,
        &fixtures.config,
        &fixtures.credential,
    )?;
    if delivery_receipt.bundle_root_hex() != delivery_root {
        bail!("V268 authenticated public-fixture delivery root drifted");
    }
    let (session_policy, session_policy_digest) = server_supervisor_session_policy_catalog()?;
    let (runner_policy, runner_policy_digest) =
        server_runtime_compatibility_runner_policy_catalog()?;
    if session_policy_digest != selected.supervisor_session_policy.policy_digest
        || runner_policy_digest != selected.runner_policy.policy_digest
        || runner_policy.max_probe_timeout_ms != RUNTIME_COMPATIBILITY_MAX_PROBE_TIMEOUT_MS
        || session_policy.state.probe_timeout_ms != runner_policy.max_probe_timeout_ms
    {
        bail!("V268 supervisor/session policy drifted during execution");
    }
    let request = receive_external_pool_adapter_no_work_probe_request(
        &mut session,
        Duration::from_millis(session_policy.state.probe_timeout_ms),
    )?;
    if request.request() != fixtures.request.as_slice()
        || request.expected_response_bytes() != fixtures.response.len()
    {
        bail!("V268 Adapter no-work request is not the exact controlled public fixture");
    }
    let no_work_receipt = request.complete(&mut session, &fixtures.response)?;
    let no_work = ExternalPoolAdapterRuntimeCompatibilityNoWorkEvidence {
        probe_nonce_digest: no_work_receipt.probe_nonce_digest_hex(),
        request_bytes: u64::from(no_work_receipt.request_bytes()),
        response_bytes: u64::from(no_work_receipt.response_bytes()),
        request_sha256: no_work_receipt.request_sha256_hex(),
        response_sha256: no_work_receipt.response_sha256_hex(),
        probe_root_sha256: no_work_receipt.probe_root_hex(),
    };
    let request_root = &selected.fixture_resources[2];
    let response_root = &selected.fixture_resources[3];
    if no_work.request_sha256 != request_root.sha256
        || no_work.request_bytes != request_root.size_bytes
        || no_work.response_sha256 != response_root.sha256
        || no_work.response_bytes != response_root.size_bytes
    {
        bail!("V268 authenticated no-work receipt roots drifted");
    }
    delivery_receipt.shutdown(&mut session)?;
    let exit = child
        .wait(CHILD_EXIT_TIMEOUT)?
        .ok_or_else(|| anyhow::anyhow!("V268 child did not exit within the bounded reap window"))?;
    if exit.exit_code != Some(0) || exit.signal.is_some() {
        bail!("V268 child failed authenticated shutdown");
    }
    if !child.collect_stderr()?.is_empty() {
        bail!("V268 child emitted stderr");
    }
    let duration_ms = u64::try_from(started.elapsed().as_millis())?;
    if duration_ms > RUNTIME_COMPATIBILITY_MAX_RUN_SECONDS * 1000 {
        bail!("V268 controlled run exceeded its server-fixed duration");
    }
    Ok(RuntimeCompatibilityExecutionEvidence {
        source_capsule_sha256: capsule.entrypoint_sha256().into(),
        source_capsule_size_bytes: capsule.entrypoint_size_bytes(),
        launch_image_sha256: capsule.launch_sha256().into(),
        launch_image_size_bytes: capsule.launch_size_bytes(),
        public_fixture_delivery_root: delivery_root,
        no_work,
        duration_ms,
    })
}
