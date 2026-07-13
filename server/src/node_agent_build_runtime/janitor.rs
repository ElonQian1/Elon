use super::{
    admission, cleanup, paths::BuildRunPaths, reservation, target_lock::AdmissionLock, telemetry,
    usage, BuildCachePolicy,
};
use std::sync::{mpsc, OnceLock};

#[derive(Debug)]
pub(crate) struct JanitorJob {
    pub(crate) paths: BuildRunPaths,
    pub(crate) policy: BuildCachePolicy,
    pub(crate) succeeded: bool,
    pub(crate) initial_reclaimed_bytes: u64,
}

/// Drop paths only enqueue a small owned message. All recursive accounting and
/// deletion runs on this dedicated worker, never on a Tokio runtime worker.
pub(crate) fn enqueue(job: JanitorJob) {
    if let Err(error) = sender().send(job) {
        tracing::warn!("构建缓存 janitor 队列不可用，启动一次性后台清理线程");
        let job = error.0;
        let _ = std::thread::Builder::new()
            .name("elon-cache-janitor-fallback".into())
            .spawn(move || process(job));
    }
}

fn sender() -> &'static mpsc::Sender<JanitorJob> {
    static SENDER: OnceLock<mpsc::Sender<JanitorJob>> = OnceLock::new();
    SENDER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<JanitorJob>();
        let started = std::thread::Builder::new()
            .name("elon-cache-janitor".into())
            .spawn(move || {
                while let Ok(job) = rx.recv() {
                    let outcome =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| process(job)));
                    if outcome.is_err() {
                        tracing::error!("构建缓存 janitor 任务 panic，工作线程继续服务后续任务");
                    }
                }
            });
        if let Err(error) = started {
            tracing::error!(error = %error, "无法启动构建缓存 janitor 工作线程");
        }
        tx
    })
}

fn process(job: JanitorJob) {
    let _admission_lock = match AdmissionLock::acquire(&job.paths, "janitor") {
        Ok(lock) => lock,
        Err(error) => {
            tracing::warn!(error = %error, "后台清理无法获取跨进程准入锁，跳过本轮");
            return;
        }
    };
    let _prepare_guard = match super::prepare_lock().lock() {
        Ok(guard) => guard,
        Err(_) => {
            tracing::warn!("构建缓存准备锁已损坏，跳过后台收尾清理");
            return;
        }
    };

    let mut report = cleanup::CleanupReport::default();
    if job.succeeded {
        match cleanup::remove_managed_path(&job.paths.root, &job.paths.task_temp) {
            Ok(reclaimed) if reclaimed > 0 => {
                report.reclaimed_bytes = report.reclaimed_bytes.saturating_add(reclaimed);
                report.removed_paths += 1;
            }
            Ok(_) => {}
            Err(error) => tracing::warn!(
                error = %error,
                task_temp = %job.paths.task_temp.display(),
                "成功任务的临时目录后台清理失败"
            ),
        }
    }
    if let Err(error) = usage::touch(&job.paths) {
        tracing::warn!(error = %error, "更新 Rust target 最后使用时间失败");
    }

    let active_reserved_bytes = match reservation::active_reserved_bytes(&job.paths) {
        Ok(bytes) => bytes,
        Err(error) => {
            tracing::warn!(error = %error, "无法可信读取活动构建预留，跳过破坏性压力清理");
            telemetry::record_cleanup(&job.paths, &report);
            super::invalidate_status(&job.paths.root);
            let snapshot = telemetry::capture(
                &job.paths,
                job.initial_reclaimed_bytes
                    .saturating_add(report.reclaimed_bytes),
                &job.policy,
            );
            telemetry::persist(&job.paths, &snapshot);
            return;
        }
    };
    if admission::under_pressure(&job.paths, &job.policy, active_reserved_bytes) {
        match cleanup::cleanup_for_pressure(&job.paths, &job.policy, active_reserved_bytes) {
            Ok(pressure_report) => report.merge(pressure_report),
            Err(error) => tracing::warn!(
                error = %error,
                project = %job.paths.project_key,
                "PC 节点任务结束后的构建盘压力清理失败"
            ),
        }
    }

    telemetry::record_cleanup(&job.paths, &report);
    super::invalidate_status(&job.paths.root);
    let reclaimed_bytes = job
        .initial_reclaimed_bytes
        .saturating_add(report.reclaimed_bytes);
    let final_snapshot = telemetry::capture(&job.paths, reclaimed_bytes, &job.policy);
    telemetry::persist(&job.paths, &final_snapshot);
    tracing::info!(
        project = %job.paths.project_key,
        reclaimed_bytes = report.reclaimed_bytes,
        removed_paths = report.removed_paths,
        skipped_active_paths = report.skipped_active_paths,
        active_reserved_bytes,
        "PC 节点构建缓存后台收尾完成"
    );
}
