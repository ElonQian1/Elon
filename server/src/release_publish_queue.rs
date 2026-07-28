//! Durable publish queue operations shared by all release lanes.

use crate::release_manager::{
    lane_mut, Lane, PublishCompletion, PublishLeaseEntry, ReleaseStateFile,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PublishAdmission {
    Owner,
    Waiter { queue_position: usize },
}

pub(crate) fn sweep_global_expired(state: &mut ReleaseStateFile, now: i64) {
    let owner_expired = state
        .global_publish
        .owner
        .as_ref()
        .is_some_and(|owner| owner.lease_expires_at <= now);
    if owner_expired {
        if let Some(owner) = state.global_publish.owner.take() {
            crate::release_batch::expire_stage(state, &owner, now);
            remove_publish_token_from_lanes(state, &owner.token);
            state.global_publish.completed.push(PublishCompletion {
                token: owner.token,
                kind: owner.kind,
                sha: owner.sha,
                batch_id: owner.batch_id,
                stage: owner.stage,
                success: false,
                coalesced: false,
                finished_at: now,
                error_message: Some("publish owner lease expired".to_string()),
            });
        }
    }
    let expired_waiters = state
        .global_publish
        .waiters
        .iter()
        .filter(|waiter| waiter.lease_expires_at <= now)
        .map(|waiter| waiter.token.clone())
        .collect::<Vec<_>>();
    state
        .global_publish
        .waiters
        .retain(|waiter| waiter.lease_expires_at > now);
    for token in expired_waiters {
        remove_publish_token_from_lanes(state, &token);
    }
    if state.global_publish.owner.is_none() && !state.global_publish.waiters.is_empty() {
        state.global_publish.owner = Some(state.global_publish.waiters.remove(0));
    }
    if state.global_publish.completed.len() > 200 {
        let keep_from = state.global_publish.completed.len() - 200;
        state.global_publish.completed.drain(..keep_from);
    }
}

pub(crate) fn enqueue_global_publish(
    state: &mut ReleaseStateFile,
    entry: PublishLeaseEntry,
) -> PublishAdmission {
    if state.global_publish.owner.is_none() {
        state.global_publish.owner = Some(entry);
        PublishAdmission::Owner
    } else {
        state.global_publish.waiters.push(entry);
        PublishAdmission::Waiter {
            queue_position: state.global_publish.waiters.len(),
        }
    }
}

pub(crate) fn finish_global_publish(
    state: &mut ReleaseStateFile,
    owner: &PublishLeaseEntry,
    success: bool,
    error_message: Option<String>,
    now: i64,
) -> Vec<String> {
    state.global_publish.owner = None;
    state.global_publish.completed.push(PublishCompletion {
        token: owner.token.clone(),
        kind: owner.kind.clone(),
        sha: owner.sha.clone(),
        batch_id: owner.batch_id.clone(),
        stage: owner.stage.clone(),
        success,
        coalesced: false,
        finished_at: now,
        error_message,
    });

    let mut coalesced_tokens = Vec::new();
    if success {
        state.global_publish.waiters.retain(|waiter| {
            let same_release = waiter.kind == owner.kind
                && waiter.sha == owner.sha
                && waiter.batch_id == owner.batch_id
                && waiter.stage == owner.stage;
            if same_release {
                coalesced_tokens.push(waiter.token.clone());
            }
            !same_release
        });
        for token in &coalesced_tokens {
            remove_publish_token_from_lanes(state, token);
            state.global_publish.completed.push(PublishCompletion {
                token: token.clone(),
                kind: owner.kind.clone(),
                sha: owner.sha.clone(),
                batch_id: owner.batch_id.clone(),
                stage: owner.stage.clone(),
                success: true,
                coalesced: true,
                finished_at: now,
                error_message: None,
            });
        }
    }
    if state.global_publish.owner.is_none() && !state.global_publish.waiters.is_empty() {
        state.global_publish.owner = Some(state.global_publish.waiters.remove(0));
    }
    coalesced_tokens
}

fn remove_publish_token_from_lanes(state: &mut ReleaseStateFile, token: &str) {
    for kind in [Lane::Server, Lane::Apk, Lane::NodeAgent] {
        lane_mut(state, kind)
            .in_flight
            .retain(|item| item.token != token);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex as StdMutex};

    fn lease(token: &str, kind: &str, sha: &str, expires: i64) -> PublishLeaseEntry {
        PublishLeaseEntry {
            token: token.to_string(),
            kind: kind.to_string(),
            sha: sha.to_string(),
            batch_id: crate::release_batch::default_batch_id_for_kind(kind, sha),
            stage: crate::release_batch::default_stage(kind).to_string(),
            builder_id: token.to_string(),
            builder_label: token.to_string(),
            requested_at: 1,
            last_heartbeat: 1,
            lease_expires_at: expires,
        }
    }

    #[test]
    fn global_publish_is_fifo_under_concurrent_claims() {
        let state = Arc::new(StdMutex::new(ReleaseStateFile::default()));
        let mut joins = Vec::new();
        for index in 0..8 {
            let state = state.clone();
            joins.push(std::thread::spawn(move || {
                let mut guard = state.lock().expect("publish test state");
                enqueue_global_publish(
                    &mut guard,
                    lease(
                        &format!("token-{index}"),
                        "server",
                        &format!("sha-{index}"),
                        100,
                    ),
                )
            }));
        }
        for join in joins {
            join.join().expect("claim thread");
        }
        let guard = state.lock().expect("publish test state");
        assert!(guard.global_publish.owner.is_some());
        assert_eq!(guard.global_publish.waiters.len(), 7);
        let requested = guard
            .global_publish
            .owner
            .iter()
            .chain(guard.global_publish.waiters.iter())
            .map(|item| item.token.clone())
            .collect::<Vec<_>>();
        assert_eq!(requested.len(), 8);
        let unique = requested.iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(unique.len(), 8);
    }

    #[test]
    fn same_sha_waiters_coalesce_and_next_distinct_sha_is_promoted() {
        let mut state = ReleaseStateFile::default();
        let owner = lease("owner", "node_agent", "sha-a", 100);
        assert_eq!(
            enqueue_global_publish(&mut state, owner.clone()),
            PublishAdmission::Owner
        );
        enqueue_global_publish(&mut state, lease("same", "node_agent", "sha-a", 100));
        enqueue_global_publish(&mut state, lease("next", "apk", "sha-b", 100));

        let coalesced = finish_global_publish(&mut state, &owner, true, None, 10);

        assert_eq!(coalesced, vec!["same"]);
        assert_eq!(state.global_publish.owner.as_ref().unwrap().token, "next");
        assert!(state
            .global_publish
            .completed
            .iter()
            .any(|item| item.token == "same" && item.coalesced));
    }

    #[test]
    fn expired_owner_yields_to_first_live_waiter() {
        let mut state = ReleaseStateFile::default();
        let expired = lease("expired", "server", "sha-a", 10);
        crate::release_batch::record_claim(&mut state, &expired, "running", 1);
        state.global_publish.owner = Some(expired);
        state.global_publish.waiters = vec![
            lease("first", "apk", "sha-b", 100),
            lease("second", "node_agent", "sha-c", 100),
        ];

        sweep_global_expired(&mut state, 11);

        assert_eq!(state.global_publish.owner.as_ref().unwrap().token, "first");
        assert_eq!(state.global_publish.waiters[0].token, "second");
        assert_eq!(state.release_batches[0].status, "failed_closed");
        assert_eq!(state.release_batches[0].stages[0].status, "expired");
    }
}
