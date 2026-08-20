#[derive(Clone, Copy)]
pub(super) struct ExternalPoolAdapterTaskWorkerCycleReport {
    pub(super) active_preparation_completed: bool,
    pub(super) eligible_rows: usize,
    pub(super) delivery_attempted: bool,
}

pub(super) fn record(report: &ExternalPoolAdapterTaskWorkerCycleReport) {
    if report.eligible_rows == 0 {
        return;
    }
    tracing::warn!(
        eligible_rows = report.eligible_rows,
        active_preparation_completed = report.active_preparation_completed,
        delivery_attempted = report.delivery_attempted,
        "external-pool Adapter task source stage observed rows without a V278 producer"
    );
}
