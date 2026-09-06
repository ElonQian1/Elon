//! Bounded FIFO for read-only CDP retrieval; queued work never grants new observation scope.
use super::{
    cdp::{self, Method},
    emit, gap, now_ms, Context, HostEvent,
};
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};

const MAX_IN_FLIGHT: usize = 8;
const MAX_JOBS: usize = 128;
const READ_TIMEOUT_MS: u64 = 10000;
const QUEUE_TIMEOUT_MS: u64 = 60000;

struct ReadJob {
    event: HostEvent,
    method: Method,
    parameters: Value,
    key: &'static str,
    request: bool,
    enqueued_at: u64,
}
struct Slot {
    started_at: u64,
    timed_out: bool,
}

#[derive(Default)]
pub(super) struct ReadScheduler {
    waiting: VecDeque<ReadJob>,
    active: BTreeMap<u64, Slot>,
    next_id: u64,
    draining: bool,
}
impl ReadScheduler {
    pub(super) fn clear_waiting(&mut self) {
        self.waiting.clear();
    }
    fn enqueue(&mut self, job: ReadJob) -> Result<(), ReadJob> {
        if self.waiting.len() + self.active.len() >= MAX_JOBS {
            return Err(job);
        }
        self.waiting.push_back(job);
        Ok(())
    }
    fn next(&mut self, now: u64) -> Option<(u64, ReadJob)> {
        if self.active.len() >= MAX_IN_FLIGHT {
            return None;
        }
        let job = self.waiting.pop_front()?;
        self.next_id += 1;
        self.active.insert(
            self.next_id,
            Slot {
                started_at: now,
                timed_out: false,
            },
        );
        Some((self.next_id, job))
    }
    fn timed_out(&mut self, now: u64) -> bool {
        let mut changed = false;
        for slot in self.active.values_mut() {
            if !slot.timed_out && now.saturating_sub(slot.started_at) > READ_TIMEOUT_MS {
                slot.timed_out = true;
                changed = true;
            }
        }
        changed
    }
}

pub(super) fn read(
    context: &Context,
    event: HostEvent,
    method: Method,
    parameters: Value,
    key: &'static str,
    request: bool,
) {
    let rejected = {
        let mut state = context.borrow_mut();
        if !state.handle.accepts(event.generation) {
            return;
        }
        state
            .reads
            .enqueue(ReadJob {
                event,
                method,
                parameters,
                key,
                request,
                enqueued_at: now_ms(),
            })
            .err()
    };
    if let Some(mut job) = rejected {
        job.event.error_code = Some("body_read_queue_limit".into());
        emit(context, job.event);
    }
    drain(context);
}

pub(super) fn drain(context: &Context) {
    {
        let mut state = context.borrow_mut();
        if state.reads.draining {
            return;
        }
        state.reads.draining = true;
        state.synchronize();
    }
    drain_inner(context);
    context.borrow_mut().reads.draining = false;
}

fn drain_inner(context: &Context) {
    let (handle, timed_out, expired_jobs) = {
        let mut state = context.borrow_mut();
        let now = now_ms();
        let timeout = state.reads.timed_out(now);
        let mut expired = Vec::new();
        while state
            .reads
            .waiting
            .front()
            .is_some_and(|job| now.saturating_sub(job.enqueued_at) > QUEUE_TIMEOUT_MS)
        {
            if let Some(job) = state.reads.waiting.pop_front() {
                expired.push(job);
            }
        }
        if !state.handle.active() {
            state.reads.clear_waiting();
        }
        (state.handle.clone(), timeout, expired)
    };
    if timed_out {
        gap(&handle, "body_read_timed_out");
    }
    for mut job in expired_jobs {
        job.event.error_code = Some("body_queue_wait_timed_out".into());
        emit(context, job.event);
    }
    if !handle.active() {
        return;
    }
    loop {
        let Some((id, mut job)) = context.borrow_mut().reads.next(now_ms()) else {
            return;
        };
        let native_id = job
            .parameters
            .get("requestId")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let valid = {
            let state = context.borrow();
            state.handle.accepts(job.event.generation)
                && native_id.as_ref().is_none_or(|native| {
                    state.request_bindings.get(native) == job.event.request_id.as_ref()
                })
        };
        if !valid {
            context.borrow_mut().reads.active.remove(&id);
            continue;
        }
        let method = job.method;
        let parameters = std::mem::take(&mut job.parameters);
        if !cdp::call(context, method, parameters, move |context, result| {
            let valid = {
                let mut state = context.borrow_mut();
                let slot = state.reads.active.remove(&id);
                let current = state.handle.accepts(job.event.generation)
                    && native_id.as_ref().is_none_or(|native| {
                        state.request_bindings.get(native) == job.event.request_id.as_ref()
                    });
                slot.map(|slot| {
                    (
                        current,
                        slot.timed_out
                            || now_ms().saturating_sub(slot.started_at) > READ_TIMEOUT_MS,
                    )
                })
            };
            if let Some((true, timeout)) = valid {
                if timeout {
                    job.event.error_code = Some("body_read_timed_out".into());
                } else {
                    decode(&mut job, result, context.borrow().config.max_body_bytes);
                }
                emit(context, job.event);
            }
            // Releasing an old-generation native slot may unblock new-generation jobs.
            drain(context);
        }) {
            context.borrow_mut().reads.active.remove(&id);
        }
    }
}

fn decode(job: &mut ReadJob, result: Result<Value, ()>, limit: usize) {
    if let Ok(value) = result {
        if value.get("base64Encoded").and_then(Value::as_bool) == Some(true) {
            job.event.error_code = Some("encoded_body_not_supported".into());
        } else if let Some(body) = value.get(job.key).and_then(Value::as_str) {
            let (body, truncated) = bounded(body, limit);
            job.event.truncated |= truncated;
            if truncated {
                job.event.error_code = Some("body_truncated".into());
            }
            if job.request {
                job.event.request_body = Some(body);
            } else {
                job.event.body = Some(body);
            }
        } else {
            job.event.error_code = Some("body_not_available".into());
        }
    } else {
        job.event.error_code = Some("body_not_available".into());
    }
}

pub(super) fn bounded(value: &str, max: usize) -> (String, bool) {
    let mut end = value.len().min(max);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    (value[..end].into(), end < value.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn job(generation: u64) -> ReadJob {
        ReadJob {
            event: HostEvent::new(generation, "resource", "https://fixture.example/chunk.js"),
            method: Method::ScriptSource,
            parameters: serde_json::json!({"scriptId":"synthetic"}),
            key: "scriptSource",
            request: false,
            enqueued_at: 0,
        }
    }
    #[test]
    fn burst_queues_after_eight_and_completion_releases_exactly_one_slot() {
        let mut queue = ReadScheduler::default();
        for _ in 0..128 {
            assert!(queue.enqueue(job(1)).is_ok());
        }
        assert!(queue.enqueue(job(1)).is_err());
        for _ in 0..8 {
            assert!(queue.next(0).is_some());
        }
        assert!(queue.next(0).is_none());
        assert_eq!(queue.waiting.len(), 120);
        queue.active.remove(&1);
        assert!(queue.next(1).is_some());
        assert_eq!(queue.active.len(), 8);
        assert_eq!(queue.waiting.len(), 119);
    }
    #[test]
    fn generation_reset_discards_waiting_but_does_not_exceed_native_concurrency() {
        let mut queue = ReadScheduler::default();
        for _ in 0..16 {
            assert!(queue.enqueue(job(1)).is_ok());
        }
        for _ in 0..8 {
            assert!(queue.next(0).is_some());
        }
        queue.clear_waiting();
        assert!(queue.waiting.is_empty());
        assert!(queue.enqueue(job(2)).is_ok());
        assert!(queue.timed_out(READ_TIMEOUT_MS + 1));
        assert!(!queue.timed_out(READ_TIMEOUT_MS + 2));
        assert!(queue.next(READ_TIMEOUT_MS + 2).is_none());
        queue.active.remove(&1);
        let (_, next) = queue.next(READ_TIMEOUT_MS + 3).unwrap();
        assert_eq!(next.event.generation, 2);
    }
}
