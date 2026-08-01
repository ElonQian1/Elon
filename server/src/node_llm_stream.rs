//! Shared collector for node LLM streams and their durable execution lease.

use std::time::Duration;

use homecli_proto::AgentToServer;
use tokio::{
    sync::mpsc,
    time::{interval, MissedTickBehavior},
};

use crate::{store::Store, types::AppState};

#[cfg(not(test))]
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
#[cfg(test)]
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(20);

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct NodeLlmStreamOutput {
    pub content: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

pub(crate) fn recover_interrupted_runs(store: &Store) -> usize {
    let interrupted = match store.mark_interrupted_started_server_node_llm_runs() {
        Ok(runs) => runs,
        Err(error) => {
            tracing::warn!(%error, "恢复服务启动前的节点推理执行证明失败");
            return 0;
        }
    };
    for run in &interrupted {
        crate::billing::release_trusted_call(
            store,
            &run.consumer_user_id,
            &run.compute_call_id,
            "released_error",
        );
    }
    if !interrupted.is_empty() {
        tracing::info!(
            count = interrupted.len(),
            "节点推理执行证明因服务器重启失败并释放预授权"
        );
    }
    interrupted.len()
}

pub(crate) fn reconcile_expired_runs(store: &Store) -> usize {
    let expired = match store.mark_expired_started_server_node_llm_runs() {
        Ok(runs) => runs,
        Err(error) => {
            tracing::warn!(%error, "节点推理过期租约终结失败");
            return 0;
        }
    };
    for run in &expired {
        crate::billing::release_trusted_call(
            store,
            &run.consumer_user_id,
            &run.compute_call_id,
            "expired_released",
        );
    }
    if !expired.is_empty() {
        tracing::warn!(
            count = expired.len(),
            "节点推理租约过期，执行证明已失败关闭并释放预授权"
        );
    }
    expired.len()
}

pub(crate) fn spawn_expired_run_reconciler(state: std::sync::Arc<AppState>) {
    let interval_secs = std::env::var("NODE_LLM_LEASE_RECONCILE_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(30)
        .max(5);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            ticker.tick().await;
            reconcile_expired_runs(&state.store);
        }
    });
}

pub(crate) async fn collect(
    store: &Store,
    compute_call_id: &str,
    expected_req_id: &str,
    rx: &mut mpsc::UnboundedReceiver<AgentToServer>,
) -> Result<NodeLlmStreamOutput, String> {
    let mut content = String::new();
    let mut heartbeat = interval(HEARTBEAT_INTERVAL);
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Skip);
    heartbeat.tick().await;

    loop {
        tokio::select! {
            message = rx.recv() => {
                match message {
                    Some(AgentToServer::LlmStreamChunk { req_id, delta }) => {
                        ensure_request_id(expected_req_id, &req_id)?;
                        content.push_str(&delta);
                    }
                    Some(AgentToServer::LlmStreamEnd {
                        req_id,
                        prompt_tokens,
                        completion_tokens,
                        ..
                    }) => {
                        ensure_request_id(expected_req_id, &req_id)?;
                        return Ok(NodeLlmStreamOutput {
                            content,
                            prompt_tokens,
                            completion_tokens,
                        });
                    }
                    Some(AgentToServer::LlmStreamError { req_id, message }) => {
                        ensure_request_id(expected_req_id, &req_id)?;
                        return Err(message);
                    }
                    Some(_) => {}
                    None => return Err("节点推理流在结束事件前断开".to_string()),
                }
            }
            _ = heartbeat.tick() => {
                match store.heartbeat_started_server_node_llm_run(compute_call_id) {
                    Ok(true) => {}
                    Ok(false) => {
                        return Err("节点推理执行租约已终止".to_string());
                    }
                    Err(error) => {
                        tracing::warn!(
                            compute_call_id,
                            %error,
                            "node LLM execution heartbeat failed"
                        );
                    }
                }
            }
        }
    }
}

fn ensure_request_id(expected: &str, actual: &str) -> Result<(), String> {
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "节点推理流请求编号不匹配：期望 {expected}，实际 {actual}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{NodeComputeRunStart, Store};

    fn running_store(call_id: &str) -> (Store, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-node-stream-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let store = Store::open(&path).unwrap();
        let user = store
            .create_user(
                &format!("stream-{}@example.com", uuid::Uuid::new_v4().simple()),
                "secret1",
                None,
                None,
            )
            .unwrap();
        store
            .start_node_compute_run(NodeComputeRunStart {
                compute_call_id: call_id,
                consumer_user_id: &user.id,
                provider_user_id: Some(&user.id),
                node_id: "stream-node",
                model_id: Some("qwen"),
                feature: "node_llm",
                usage_mode: "server_node_llm",
                route_reason: None,
            })
            .unwrap();
        (store, path)
    }

    #[tokio::test]
    async fn collects_terminal_usage_and_keeps_long_stream_alive() {
        let call_id = "node_llm:stream-success";
        let (store, path) = running_store(call_id);
        let before = store
            .get_node_compute_run_by_compute_call_id(call_id)
            .unwrap()
            .unwrap()
            .updated_at;
        let (tx, mut rx) = mpsc::unbounded_channel();
        let sender = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1_100)).await;
            tx.send(AgentToServer::LlmStreamChunk {
                req_id: "req-success".into(),
                delta: "hello".into(),
            })
            .unwrap();
            tx.send(AgentToServer::LlmStreamEnd {
                req_id: "req-success".into(),
                prompt_tokens: 3,
                completion_tokens: 2,
                finish_reason: "stop".into(),
            })
            .unwrap();
        });

        let output = collect(&store, call_id, "req-success", &mut rx)
            .await
            .unwrap();
        sender.await.unwrap();
        assert_eq!(output.content, "hello");
        assert_eq!(output.prompt_tokens, 3);
        assert_eq!(output.completion_tokens, 2);
        let after = store
            .get_node_compute_run_by_compute_call_id(call_id)
            .unwrap()
            .unwrap()
            .updated_at;
        assert_ne!(after, before);
        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn rejects_disconnect_before_terminal_event() {
        let call_id = "node_llm:stream-disconnect";
        let (store, path) = running_store(call_id);
        let (tx, mut rx) = mpsc::unbounded_channel();
        drop(tx);
        let error = collect(&store, call_id, "req-disconnect", &mut rx)
            .await
            .unwrap_err();
        assert!(error.contains("结束事件前断开"));
        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn rejects_mismatched_terminal_request_id() {
        let call_id = "node_llm:stream-mismatch";
        let (store, path) = running_store(call_id);
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send(AgentToServer::LlmStreamEnd {
            req_id: "another-request".into(),
            prompt_tokens: 1,
            completion_tokens: 1,
            finish_reason: "stop".into(),
        })
        .unwrap();
        let error = collect(&store, call_id, "expected-request", &mut rx)
            .await
            .unwrap_err();
        assert!(error.contains("请求编号不匹配"));
        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn restart_recovery_releases_reserved_balance() {
        let call_id = "node_llm:restart-refund";
        let (store, path) = running_store(call_id);
        let run = store
            .get_node_compute_run_by_compute_call_id(call_id)
            .unwrap()
            .unwrap();
        store
            .billing_recharge(&run.consumer_user_id, 1_000, "test", "test", None)
            .unwrap();
        crate::billing::reserve_trusted_call(
            &store,
            &run.consumer_user_id,
            call_id,
            "node_llm",
            "server_node_llm",
            Some("qwen"),
            100,
        )
        .unwrap();
        assert_eq!(
            store.billing_get_balance(&run.consumer_user_id).unwrap(),
            Some(900)
        );
        assert!(store
            .mark_server_node_llm_usage_received(call_id, run.provider_user_id.as_deref(), 2, 1,)
            .unwrap());

        assert_eq!(recover_interrupted_runs(&store), 1);
        assert_eq!(
            store.billing_get_balance(&run.consumer_user_id).unwrap(),
            Some(1_000)
        );
        assert!(!store
            .billing_reservation_is_still_reserved(&run.consumer_user_id, call_id)
            .unwrap());
        let recovered = store
            .get_node_compute_run_by_compute_call_id(call_id)
            .unwrap()
            .unwrap();
        assert_eq!(recovered.status, "failed");
        assert_eq!(
            recovered.settlement_status.as_deref(),
            Some("released_error")
        );
        assert_eq!(
            recovered.error_message.as_deref(),
            Some("server restarted before node LLM settlement completed")
        );
        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn expired_reconcile_releases_reserved_balance_once() {
        let call_id = "node_llm:expired-refund";
        let (store, path) = running_store(call_id);
        let run = store
            .get_node_compute_run_by_compute_call_id(call_id)
            .unwrap()
            .unwrap();
        store
            .billing_recharge(&run.consumer_user_id, 1_000, "test", "test", None)
            .unwrap();
        crate::billing::reserve_trusted_call(
            &store,
            &run.consumer_user_id,
            call_id,
            "node_llm",
            "server_node_llm",
            Some("qwen"),
            100,
        )
        .unwrap();
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute(
            "UPDATE node_compute_runs SET updated_at='2000-01-01T00:00:00Z'
              WHERE compute_call_id=?1",
            rusqlite::params![call_id],
        )
        .unwrap();
        drop(conn);

        assert_eq!(reconcile_expired_runs(&store), 1);
        assert_eq!(reconcile_expired_runs(&store), 0);
        assert_eq!(
            store.billing_get_balance(&run.consumer_user_id).unwrap(),
            Some(1_000)
        );
        assert!(!store
            .billing_reservation_is_still_reserved(&run.consumer_user_id, call_id)
            .unwrap());
        let reconciled = store
            .get_node_compute_run_by_compute_call_id(call_id)
            .unwrap()
            .unwrap();
        assert_eq!(reconciled.status, "failed");
        assert_eq!(
            reconciled.settlement_status.as_deref(),
            Some("expired_released")
        );

        drop(store);
        let _ = std::fs::remove_file(path);
    }
}
