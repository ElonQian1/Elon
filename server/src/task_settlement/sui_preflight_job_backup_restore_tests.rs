use std::path::{Path, PathBuf};

use crate::store::Store;

use super::{
    sui_preflight_job_model::{
        ClaimSuiPreflightJobRequest, CompleteSuiPreflightJobRequest, QueueSuiPreflightJobRequest,
        SuiPreflightJob, SuiPreflightJobIssue,
    },
    sui_preflight_job_service as service,
    sui_preflight_job_test_support::{fixture, RuntimeFlagGuard, SuiPreflightJobFixture},
    sui_preflight_model::SuiPreflightAdapter,
    sui_preflight_service,
};

#[test]
fn queued_job_survives_local_backup_and_can_complete_after_restore() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = fixture();
    let queued = queue(&fixture);
    let backup_path = local_backup(&fixture.store, &fixture.path, "queued");
    let restored = Store::open(&backup_path).unwrap();
    let restored_adapter = restored
        .authenticate_task_sui_preflight_adapter(&fixture.adapter_token)
        .unwrap();

    let jobs = service::list(&restored, &fixture.project_id).unwrap();
    assert_eq!(jobs.jobs.len(), 1);
    assert_eq!(jobs.jobs[0].id, queued.id);
    assert_eq!(jobs.jobs[0].status, "pending");

    let issue = claim(&restored, &restored_adapter);
    let completed = service::complete(
        &restored,
        &restored_adapter,
        &queued.id,
        &completion(issue.lease_token, "backup-queued-complete"),
    )
    .unwrap();
    assert_eq!(completed.job.status, "completed");
    assert_eq!(
        completed.report.projection_package_id,
        fixture.projection_id
    );
}

#[test]
fn completed_job_and_report_survive_backup_with_idempotent_replay() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = fixture();
    let queued = queue(&fixture);
    let issue = claim(&fixture.store, &fixture.adapter);
    let request = completion(issue.lease_token, "backup-completed-replay");
    let completed =
        service::complete(&fixture.store, &fixture.adapter, &queued.id, &request).unwrap();
    let backup_path = local_backup(&fixture.store, &fixture.path, "completed");
    let restored = Store::open(&backup_path).unwrap();
    let restored_adapter = restored
        .authenticate_task_sui_preflight_adapter(&fixture.adapter_token)
        .unwrap();

    let restored_jobs = service::list(&restored, &fixture.project_id).unwrap();
    assert_eq!(restored_jobs.jobs.len(), 1);
    assert_eq!(restored_jobs.jobs[0].status, "completed");
    assert_eq!(
        restored_jobs.jobs[0].report_id.as_deref(),
        Some(completed.report.id.as_str())
    );
    let reports = sui_preflight_service::list_reports(&restored, &fixture.project_id).unwrap();
    assert_eq!(reports.reports.len(), 1);
    assert_eq!(reports.reports[0].id, completed.report.id);

    let replay = service::complete(&restored, &restored_adapter, &queued.id, &request).unwrap();
    assert_eq!(replay.report.id, completed.report.id);
    assert_eq!(
        sui_preflight_service::list_reports(&restored, &fixture.project_id)
            .unwrap()
            .reports
            .len(),
        1
    );
}

fn local_backup(store: &Store, source_path: &Path, label: &str) -> PathBuf {
    let backup_path = source_path.with_extension(format!("{label}.backup.sqlite"));
    let _ = std::fs::remove_file(&backup_path);
    let backup_text = backup_path.to_string_lossy().into_owned();
    store
        .conn()
        .unwrap()
        .execute("VACUUM INTO ?1", [&backup_text])
        .unwrap();
    backup_path
}

fn queue(fixture: &SuiPreflightJobFixture) -> SuiPreflightJob {
    service::queue(
        &fixture.store,
        &fixture.project_id,
        &fixture.owner_user_id,
        "owner",
        &QueueSuiPreflightJobRequest {
            package_kind: "standard".to_string(),
            projection_package_id: fixture.projection_id.clone(),
            confirmed_by_user: true,
        },
    )
    .unwrap()
}

fn claim(store: &Store, adapter: &SuiPreflightAdapter) -> SuiPreflightJobIssue {
    service::claim_next(
        store,
        adapter,
        &ClaimSuiPreflightJobRequest { lease_seconds: 300 },
    )
    .unwrap()
    .issue
    .unwrap()
}

fn completion(lease_token: String, idempotency_key: &str) -> CompleteSuiPreflightJobRequest {
    CompleteSuiPreflightJobRequest {
        lease_token,
        outcome: "passed".to_string(),
        summary: "offline package verified before local backup".to_string(),
        tool_version: "backup-restore-test-v1".to_string(),
        idempotency_key: idempotency_key.to_string(),
    }
}
