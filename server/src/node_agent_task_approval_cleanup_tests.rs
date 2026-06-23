use crate::node_agent_tool_approval::{ToolApprovalDecision, ToolApprovalState};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_terminal_task_cleanup_drops_only_that_tasks_approvals() {
    let state = ToolApprovalState::default();
    let task_count = 96;
    let approvals_per_task = 3;
    let mut cleared_waiters = Vec::new();
    let mut survivor_waiters = Vec::new();

    for task_index in 0..task_count {
        let req_id = format!("req-cleanup-{task_index:03}");
        for approval_index in 0..approvals_per_task {
            let approval_id = format!("tap_{task_index}_{approval_index}");
            let waiter = state.register(&req_id, &approval_id).await;
            if task_index % 2 == 0 {
                cleared_waiters.push((req_id.clone(), approval_id, waiter));
            } else {
                survivor_waiters.push((req_id.clone(), approval_id, waiter));
            }
        }
    }

    let removed = Arc::new(AtomicUsize::new(0));
    let mut workers = Vec::new();
    for task_index in (0..task_count).step_by(2) {
        let state = state.clone();
        let removed = Arc::clone(&removed);
        workers.push(tokio::spawn(async move {
            let req_id = format!("req-cleanup-{task_index:03}");
            removed.fetch_add(state.clear_req(&req_id).await, Ordering::SeqCst);
        }));
    }
    for worker in workers {
        worker
            .await
            .expect("approval cleanup worker should not panic");
    }
    assert_eq!(
        removed.load(Ordering::SeqCst),
        (task_count / 2) * approvals_per_task
    );

    for task_index in 0..task_count {
        let req_id = format!("req-cleanup-{task_index:03}");
        let pending = state.pending_for_req(&req_id).await;
        if task_index % 2 == 0 {
            assert!(
                pending.is_empty(),
                "terminal cleanup should remove approvals for {req_id}"
            );
        } else {
            assert_eq!(
                pending.len(),
                approvals_per_task,
                "terminal cleanup must not remove approvals from live task {req_id}"
            );
        }
    }

    for (_req_id, _approval_id, mut waiter) in cleared_waiters {
        assert!(
            !waiter.changed().await,
            "cleared approval waiters should observe a closed channel"
        );
        assert_eq!(waiter.decision(), None);
    }

    for (req_id, approval_id, mut waiter) in survivor_waiters {
        assert!(state.decide(&req_id, &approval_id, "approve").await);
        assert!(waiter.changed().await);
        assert_eq!(waiter.decision(), Some(ToolApprovalDecision::Approve));
    }
}
