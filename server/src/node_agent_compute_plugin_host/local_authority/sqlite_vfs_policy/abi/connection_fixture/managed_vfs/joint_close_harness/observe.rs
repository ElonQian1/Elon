//! Capture, validate, and seal all first-xClose evidence before the retry proof.

use anyhow::{anyhow, Context};

use super::{
    super::a2b2_cases::{JointCloseActual, JointCloseSelector as S},
    action,
    boundary::{self, BoundaryEvidence},
    counts, custody,
    invoke::FirstClose,
    outcome,
    prepare::JointCloseFixture,
    runtime, shm,
};

pub(super) fn seal_after_first(
    fixture: &JointCloseFixture,
    selector: S,
    first: FirstClose,
) -> anyhow::Result<JointCloseActual> {
    let callbacks = fixture
        .owner()
        .callback_fault_observations()
        .map_err(anyhow::Error::msg)?;
    let callbacks = callbacks
        .strip_prefix(fixture.callback_baseline.as_slice())
        .context("JointClose callback observation baseline changed")?;
    let lifecycle = fixture
        .owner()
        .lifecycle_fault_observations()
        .map_err(anyhow::Error::msg)?;
    let lifecycle = lifecycle
        .strip_prefix(fixture.lifecycle_baseline.as_slice())
        .context("JointClose lifecycle observation baseline changed")?;
    let trace = fixture.route_observer.trace()?;
    if trace.pending_controls() != 0 || trace.receipt_custody_count() != 0 {
        return Err(anyhow!(
            "JointClose first xClose leaked a registry-lifecycle side control"
        ));
    }
    let runtime_trace = fixture
        .owner()
        .unmap_runtime_trace(super::prepare::SELECTED)?;
    runtime::validate(selector, &runtime_trace, trace.stages())?;

    let (shm_observed, physical) = if outcome::observes_physical_actions(selector) {
        let receipt = fixture
            .binding
            .finish_unmap_test_receipt()
            .map_err(anyhow::Error::msg)?;
        if outcome::is_shm(selector) {
            let (observed, physical) = shm::validate(&fixture.binding, selector, &receipt)?;
            (Some(observed), physical)
        } else {
            (None, action::validate_complete(&receipt)?)
        }
    } else {
        (None, action::JointClosePhysicalObserved::NONE)
    };
    let post_physical = fixture
        .binding
        .observer()
        .map_err(anyhow::Error::msg)?
        .snapshot()
        .map_err(|failure| anyhow!("observe JointClose post physical state: {failure:?}"))?;
    let terminal = fixture.route_observer.terminal_custody()?;
    let control = if has_control(selector) {
        Some(
            fixture
                .lifecycle
                .joint_close_control_snapshot()
                .map_err(anyhow::Error::msg)?,
        )
    } else {
        None
    };
    let callback_claims = if selector == S::CallbackAdmissionRejected {
        Some(
            fixture
                .route_observer
                .close_callback_admission_claim_count()?,
        )
    } else {
        None
    };
    let begin_claims = if selector == S::BeginConnectionCloseRejected {
        Some(
            fixture
                .route_observer
                .begin_connection_close_claim_count()?,
        )
    } else {
        None
    };
    let registry_claims = if selector == S::RegistryWalMainCloseNativeUncertain {
        Some(
            fixture
                .route_observer
                .registry_wal_main_native_uncertain_claim_count()?,
        )
    } else {
        None
    };
    let sealed = boundary::seal(BoundaryEvidence {
        selector,
        code: first.code,
        route: fixture.route,
        callbacks,
        lifecycle,
        stages: trace.stages(),
        control,
        shm: shm_observed,
        custody: terminal,
        callback_claims,
        registry_claims,
        begin_claims,
        callback_pending: fixture
            .owner()
            .pending_callback_fault_count()
            .map_err(anyhow::Error::msg)?,
        lifecycle_pending: fixture
            .owner()
            .pending_lifecycle_fault_count()
            .map_err(anyhow::Error::msg)?,
        generic_pending: fixture
            .binding
            .pending_count()
            .map_err(anyhow::Error::msg)?,
    })?;
    if sealed.selector() != selector {
        return Err(anyhow!(
            "JointClose sealed selector changed during validation"
        ));
    }
    let observed_custody = custody::validate_and_project(fixture, sealed, post_physical, terminal)?;
    let observed_counts = counts::validate_and_project(
        selector,
        sealed,
        first.raw,
        trace.stages(),
        shm_observed,
        physical,
        terminal.route_removal_count(),
    )?;
    Ok(JointCloseActual {
        selector,
        identity: outcome::identity(sealed, fixture.target),
        mutation_may_have_occurred: sealed.mutation_may_have_occurred(),
        lock_outcome_uncertain: sealed.lock_outcome_uncertain(),
        domain_terminal: sealed.domain_terminal(),
        registry_route_phase: sealed.registry_route_phase(),
        logical_route_phase: sealed.logical_route_phase(),
        registration_phase: super::super::a2b2_cases::JointCloseRegistrationPhase::Registered,
        later_callback_allowed: sealed.later_callback_allowed(),
        pre: fixture.pre_topology(),
        post: observed_custody.post,
        retained: observed_custody.retained,
        counts: observed_counts,
    })
}

fn has_control(selector: S) -> bool {
    matches!(
        selector,
        S::BeginConnectionCloseRejected
            | S::CallbackAdmissionRejected
            | S::MainLockReleaseNativeUncertainShared
            | S::MainLockReleaseNativeUncertainReserved
            | S::MainFileCloseNativeRetryable
            | S::MainFileCloseNativeUncertain
            | S::PhysicalSuccess
            | S::RegistryWalMainCloseNativeUncertain
    )
}
