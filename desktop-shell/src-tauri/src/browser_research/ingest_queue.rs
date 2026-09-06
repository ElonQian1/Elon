//! Nonblocking capture ingress with a shared count and estimated-memory budget.
use super::host::HostEvent;
use serde_json::Value;
use std::{
    io::{self, Write},
    mem::size_of,
    sync::{mpsc, Arc, Mutex},
};

const MAX_EVENTS: usize = 256;
const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_INITIATOR_DEPTH: usize = 64;

#[derive(Default)]
struct Budget {
    events: usize,
    bytes: usize,
}

struct Reservation {
    budget: Arc<Mutex<Budget>>,
    bytes: usize,
}

impl Drop for Reservation {
    fn drop(&mut self) {
        let mut budget = self.budget.lock().unwrap_or_else(|e| e.into_inner());
        budget.events -= 1;
        budget.bytes -= self.bytes;
    }
}

struct QueuedEvent {
    event: HostEvent,
    reservation: Reservation,
}

#[derive(Clone)]
pub(super) struct Sender {
    tx: mpsc::SyncSender<QueuedEvent>,
    budget: Arc<Mutex<Budget>>,
    max_events: usize,
    max_bytes: usize,
}

pub(super) struct Receiver(mpsc::Receiver<QueuedEvent>);

pub(super) fn channel() -> (Sender, Receiver) {
    with_limits(MAX_EVENTS, MAX_BYTES)
}

fn with_limits(max_events: usize, max_bytes: usize) -> (Sender, Receiver) {
    let (tx, rx) = mpsc::sync_channel(max_events);
    (
        Sender {
            tx,
            budget: Arc::new(Mutex::new(Budget::default())),
            max_events,
            max_bytes,
        },
        Receiver(rx),
    )
}

impl Sender {
    /// False means count/byte pressure, unsupported metadata depth, or a closed receiver.
    /// Rejected events are dropped; callers must record their existing coverage gap.
    pub(super) fn try_send(&self, event: HostEvent) -> bool {
        let Some(bytes) = estimate(&event, self.max_bytes) else {
            return false;
        };
        {
            let mut budget = self.budget.lock().unwrap_or_else(|e| e.into_inner());
            if budget.events >= self.max_events || bytes > self.max_bytes - budget.bytes {
                return false;
            }
            budget.events += 1;
            budget.bytes += bytes;
        }
        // Failed sends drop the returned envelope, releasing both reservations.
        self.tx
            .try_send(QueuedEvent {
                event,
                reservation: Reservation {
                    budget: self.budget.clone(),
                    bytes,
                },
            })
            .is_ok()
    }
}

impl Receiver {
    pub(super) fn recv(&self) -> Result<HostEvent, mpsc::RecvError> {
        let QueuedEvent { event, reservation } = self.0.recv()?;
        // The worker owns this event now; its processing time must not hold queue capacity.
        drop(reservation);
        Ok(event)
    }
}

struct Estimate {
    bytes: usize,
    limit: usize,
}

impl Estimate {
    fn add(&mut self, bytes: usize) -> Option<()> {
        self.bytes = self.bytes.checked_add(bytes)?;
        (self.bytes <= self.limit).then_some(())
    }

    // Account for the DOM as well as serialized bytes, without allocating a JSON copy.
    // Bound depth before invoking the recursive serializer on untrusted metadata.
    fn value(&mut self, value: &Value, depth: usize) -> Option<()> {
        if depth > MAX_INITIATOR_DEPTH {
            return None;
        }
        self.add(size_of::<Value>())?;
        match value {
            Value::String(text) => self.add(text.capacity())?,
            Value::Array(items) => {
                self.add(items.capacity().checked_mul(size_of::<Value>())?)?;
                for item in items {
                    self.value(item, depth + 1)?;
                }
            }
            Value::Object(items) => {
                for (key, value) in items {
                    self.add(key.capacity())?;
                    self.add(size_of::<String>() + 3 * size_of::<usize>())?;
                    self.value(value, depth + 1)?;
                }
            }
            _ => {}
        }
        Some(())
    }
}

impl Write for Estimate {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.add(bytes.len())
            .ok_or_else(|| io::Error::other("research_event_budget"))?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn estimate(event: &HostEvent, limit: usize) -> Option<usize> {
    let mut estimate = Estimate { bytes: 0, limit };
    estimate.add(size_of::<QueuedEvent>())?;
    for text in [
        Some(&event.kind),
        Some(&event.url),
        event.method.as_ref(),
        event.resource_type.as_ref(),
        event.request_id.as_ref(),
        event.script_id.as_ref(),
        event.request_body.as_ref(),
        event.body.as_ref(),
        event.mime.as_ref(),
        event.error_code.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        // Capacity includes preallocated but unused bytes, unlike string length.
        estimate.add(text.capacity())?;
    }
    if let Some(value) = &event.initiator {
        estimate.value(value, 0)?;
        serde_json::to_writer(&mut estimate, value).ok()?;
    }
    Some(estimate.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(body_bytes: usize) -> HostEvent {
        HostEvent {
            generation: 1,
            kind: "resource".into(),
            url: "https://fixture.invalid/asset.js".into(),
            method: None,
            status: None,
            resource_type: None,
            request_id: None,
            script_id: None,
            request_body: None,
            body: Some("x".repeat(body_bytes)),
            mime: None,
            initiator: None,
            truncated: false,
            error_code: None,
        }
    }

    fn usage(sender: &Sender) -> (usize, usize) {
        let budget = sender.budget.lock().unwrap();
        (budget.events, budget.bytes)
    }

    #[test]
    fn metadata_burst_accepts_256_then_rejects_and_recovers() {
        let (tx, rx) = channel();
        for _ in 0..256 {
            assert!(tx.try_send(event(0)));
        }
        assert!(!tx.try_send(event(0)));
        assert_eq!(usage(&tx).0, 256);
        rx.recv().unwrap();
        assert!(tx.try_send(event(0)));
        assert_eq!(usage(&tx).0, 256);
    }

    #[test]
    fn mixed_bodies_respect_shared_byte_budget_and_recv_releases_it() {
        let small = event(128);
        let large = event(16 * 1024);
        let small_size = estimate(&small, MAX_BYTES).unwrap();
        let limit = small_size + estimate(&large, MAX_BYTES).unwrap();
        let (tx, rx) = with_limits(256, limit);
        assert!(tx.try_send(small));
        assert!(tx.clone().try_send(large));
        assert_eq!(usage(&tx), (2, limit));
        assert!(!tx.try_send(event(1)));
        let processing = rx.recv().unwrap();
        assert_eq!(processing.body.as_ref().unwrap().len(), 128);
        assert_eq!(usage(&tx), (1, limit - small_size));
        assert!(tx.try_send(event(128)));
        assert_eq!(usage(&tx), (2, limit));
    }

    #[test]
    fn oversized_single_event_does_not_reserve_or_block_metadata() {
        let (tx, _rx) = with_limits(256, 4096);
        assert!(!tx.try_send(event(4096)));
        assert_eq!(usage(&tx), (0, 0));
        assert!(tx.try_send(event(0)));
    }

    #[test]
    fn dropping_receiver_releases_backlog_and_rejected_sends() {
        let (tx, rx) = channel();
        assert!(tx.try_send(event(128)));
        assert!(tx.clone().try_send(event(512)));
        drop(rx);
        assert_eq!(usage(&tx), (0, 0));
        assert!(!tx.try_send(event(128)));
        assert_eq!(usage(&tx), (0, 0));
    }

    #[test]
    fn sender_clones_share_limits_across_threads() {
        let (tx, rx) = channel();
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let tx = tx.clone();
                std::thread::spawn(move || (0..64).filter(|_| tx.try_send(event(0))).count())
            })
            .collect();
        let accepted: usize = threads.into_iter().map(|t| t.join().unwrap()).sum();
        assert_eq!(accepted, 256);
        assert_eq!(usage(&tx).0, 256);
        drop(rx);
        assert_eq!(usage(&tx), (0, 0));
    }

    #[test]
    fn all_owned_metadata_and_spare_capacity_are_budgeted() {
        let mut sample = event(0);
        let base = estimate(&sample, MAX_BYTES).unwrap();
        sample.request_body = Some("r".repeat(500));
        sample.script_id = Some("s".repeat(500));
        sample.request_id = Some("i".repeat(500));
        sample.mime = Some("m".repeat(500));
        sample.method = Some("GET".into());
        sample.url.reserve(2000);
        assert!(estimate(&sample, MAX_BYTES).unwrap() >= base + 4000);
        let (tx, _rx) = with_limits(256, base + 3999);
        assert!(!tx.try_send(sample));
        assert_eq!(usage(&tx), (0, 0));
    }

    #[test]
    fn initiator_serialization_is_bounded_without_retaining_a_json_copy() {
        let mut sample = event(0);
        sample.initiator = Some(serde_json::json!({"stack": "\n".repeat(2048)}));
        let (tx, _rx) = with_limits(256, 4096);
        assert!(!tx.try_send(sample));
        assert_eq!(usage(&tx), (0, 0));
        let mut deep = Value::Null;
        for _ in 0..=MAX_INITIATOR_DEPTH {
            deep = Value::Array(vec![deep]);
        }
        let mut sample = event(0);
        sample.initiator = Some(deep);
        assert!(!tx.try_send(sample));
        assert_eq!(usage(&tx), (0, 0));
    }
}
