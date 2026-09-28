use serde::Serialize;
#[path = "../../../../src/node_agent_android_live/build_verify/gradle.rs"]
mod gradle;
#[path = "../../../../src/node_agent_android_live/build_verify/preparation.rs"]
mod preparation;
pub(crate) use preparation::PreparationRegistry;

// Only external runtime/build contracts are substituted. Registry, source Git
// integration, atomic files, routing and OS-backed deployment leases are real.
#[derive(Clone)]
pub(crate) struct Lease {
    pub project_id: String,
    pub hardware_serial: String,
}
#[derive(Clone)]
pub(crate) struct PrepareDebugRuntimeRequest {
    pub device_id: String,
    pub base_package_name: String,
    pub project_root: String,
    pub debug_application_id_suffix: String,
    pub isolated_emulator_package: bool,
    pub lkg_enabled: bool,
    pub candidate: Option<super::debug_integration::DebugMergeCandidateRequest>,
    pub lease: Option<Lease>,
    pub integration_plan: Option<super::debug_integration::DebugIntegrationPlan>,
}
#[derive(Debug, Clone, Serialize)]
pub(crate) struct PrepareDebugRuntimeResult {
    pub build: BuildResult,
    pub integration: super::debug_integration::DebugIntegrationStatus,
}
#[derive(Debug, Clone, Serialize)]
pub(crate) struct BuildResult {
    pub runtime_connected: bool,
    pub runtime_build_id: Option<String>,
    pub node_count: usize,
}
async fn bootstrap_debug_runtime_with_reporter(
    broker: &super::broker::LiveUiBroker,
    request: PrepareDebugRuntimeRequest,
    _: u16,
    reporter: Option<&preparation::PreparationReporter>,
) -> anyhow::Result<PrepareDebugRuntimeResult> {
    let plan = request.integration_plan.unwrap();
    let _lease = broker
        .debug_deployments
        .acquire(&request.device_id, &plan.package_name)
        .await?;
    broker.debug_integration.materialize(&plan)?;
    let session = match broker
        .runtime_session(&request.device_id, &plan.package_name)
        .await
    {
        Some(session) => session,
        None => {
            broker
                .create_session(
                    request.device_id,
                    plan.package_name.clone(),
                    Some(request.project_root),
                    1,
                )
                .await
        }
    };
    if let Some(reporter) = reporter {
        reporter.bind_runtime(&session.id).await;
    }
    let error = anyhow::anyhow!("fault-injected external build failure");
    broker
        .debug_integration
        .record_runtime_failure(&plan, error.to_string())?;
    Err(error)
}
