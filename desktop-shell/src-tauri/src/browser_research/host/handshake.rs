//! Bounded native attachment and fixed CDP acknowledgements, independent of the UI pump.
use super::types::{HostEvent, HostHandle};
use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Stage {
    NativeDispatch,
    NativeCreate,
    NativeAttached,
    PageEnable,
    FrameTree,
    RuntimeReset,
    RuntimeEnable,
    DebuggerReset,
    NetworkEnable,
    DebuggerSafeMode,
    DebuggerEnable,
}
impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::NativeDispatch => "native_dispatch",
            Self::NativeCreate => "native_create",
            Self::NativeAttached => "native_attached",
            Self::PageEnable => "page_enable",
            Self::FrameTree => "frame_tree",
            Self::RuntimeReset => "runtime_reset",
            Self::RuntimeEnable => "runtime_enable",
            Self::DebuggerReset => "debugger_reset",
            Self::NetworkEnable => "network_enable",
            Self::DebuggerSafeMode => "debugger_safe_mode",
            Self::DebuggerEnable => "debugger_enable",
        }
    }
    fn timeout(self) -> &'static str {
        match self {
            Self::NativeDispatch => "host_dispatch_timed_out",
            Self::NativeCreate | Self::NativeAttached => "host_attach_timed_out",
            Self::PageEnable => "page_enable_timed_out",
            Self::FrameTree => "frame_tree_timed_out",
            Self::RuntimeReset => "runtime_reset_timed_out",
            Self::RuntimeEnable => "runtime_enable_timed_out",
            Self::DebuggerReset => "debugger_reset_timed_out",
            Self::NetworkEnable => "network_enable_timed_out",
            Self::DebuggerSafeMode => "debugger_safe_mode_timed_out",
            Self::DebuggerEnable => "debugger_enable_timed_out",
        }
    }
}

struct Pending {
    generation: u64,
    stage: Stage,
    started: Instant,
    entered: Instant,
}
#[derive(Default)]
pub(super) struct Handshake {
    pending: Option<Pending>,
}
impl Handshake {
    fn begin(&mut self, generation: u64, now: Instant) {
        self.pending = Some(Pending {
            generation,
            stage: Stage::NativeDispatch,
            started: now,
            entered: now,
        });
    }
    fn current(&self, generation: u64) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|p| p.generation == generation)
    }
    fn advance(&mut self, generation: u64, stage: Stage, now: Instant) -> bool {
        let Some(pending) = self.pending.as_mut().filter(|p| p.generation == generation) else {
            return false;
        };
        pending.stage = stage;
        pending.entered = now;
        true
    }
    fn timeout(&self, generation: u64, now: Instant) -> Option<&'static str> {
        let pending = self
            .pending
            .as_ref()
            .filter(|p| p.generation == generation)?;
        if now.duration_since(pending.started) >= Duration::from_secs(45) {
            Some("host_handshake_timed_out")
        } else if now.duration_since(pending.entered) >= Duration::from_secs(15) {
            Some(pending.stage.timeout())
        } else {
            None
        }
    }
}

impl HostHandle {
    pub(super) fn begin_handshake(&self, resume: bool) -> u64 {
        let mut state = self.control.handshake.lock().unwrap();
        let generation = if resume {
            self.control.generation.fetch_add(1, Ordering::SeqCst) + 1
        } else {
            self.generation()
        };
        self.control.active.store(true, Ordering::SeqCst);
        state.begin(generation, Instant::now());
        let mut event = HostEvent::new(generation, "phase", "");
        event.resource_type = Some(Stage::NativeDispatch.name().into());
        (self.control.sink)(event);
        generation
    }
    pub(super) fn handshake_current(&self, generation: u64) -> bool {
        self.accepts(generation) && self.control.handshake.lock().unwrap().current(generation)
    }
    pub(super) fn handshake_pending(&self) -> bool {
        self.control
            .handshake
            .lock()
            .unwrap()
            .current(self.generation())
    }
    pub(super) fn handshake_stage(&self, generation: u64, stage: Stage) -> bool {
        let mut state = self.control.handshake.lock().unwrap();
        if !self.accepts(generation) || !state.advance(generation, stage, Instant::now()) {
            return false;
        }
        let mut event = HostEvent::new(generation, "phase", "");
        event.resource_type = Some(stage.name().into());
        (self.control.sink)(event);
        true
    }
    pub(super) fn finish_handshake(&self, generation: u64) -> bool {
        let mut state = self.control.handshake.lock().unwrap();
        if !self.accepts(generation) || !state.current(generation) {
            return false;
        }
        state.pending = None;
        let mut event = HostEvent::new(generation, "ready", "");
        event.resource_type = Some("top_frame_cdp_network_and_script".into());
        (self.control.sink)(event);
        true
    }
    pub(super) fn fail_handshake(&self, generation: u64, code: &'static str) {
        let mut state = self.control.handshake.lock().unwrap();
        if !self.accepts(generation) || !state.current(generation) {
            return;
        }
        state.pending = None;
        self.control.active.store(false, Ordering::SeqCst);
        let mut event = HostEvent::new(generation, "failed", "");
        event.error_code = Some(code.into());
        (self.control.sink)(event);
    }
    pub(super) fn poll_handshake(&self) {
        let mut state = self.control.handshake.lock().unwrap();
        let generation = self.generation();
        if !self.accepts(generation) {
            return;
        }
        let Some(code) = state.timeout(generation, Instant::now()) else {
            return;
        };
        state.pending = None;
        self.control.active.store(false, Ordering::SeqCst);
        let mut event = HostEvent::new(generation, "failed", "");
        event.error_code = Some(code.into());
        (self.control.sink)(event);
    }
    pub(super) fn navigation_during_handshake(&self) {
        let mut state = self.control.handshake.lock().unwrap();
        let generation = self.generation();
        if !self.active()
            || state
                .pending
                .as_ref()
                .is_none_or(|p| p.generation == generation)
        {
            return;
        }
        state.pending = None;
        self.control.active.store(false, Ordering::SeqCst);
        let mut event = HostEvent::new(generation, "failed", "");
        event.error_code = Some("host_handshake_navigation_changed".into());
        (self.control.sink)(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_dispatch_and_missing_ack_have_distinct_finite_deadlines() {
        let now = Instant::now();
        let mut state = Handshake::default();
        state.begin(1, now);
        assert_eq!(state.timeout(1, now + Duration::from_secs(14)), None);
        assert_eq!(
            state.timeout(1, now + Duration::from_secs(15)),
            Some("host_dispatch_timed_out")
        );
        assert!(state.advance(1, Stage::PageEnable, now + Duration::from_secs(3)));
        assert_eq!(
            state.timeout(1, now + Duration::from_secs(18)),
            Some("page_enable_timed_out")
        );
    }
    #[test]
    fn progressing_steps_cannot_extend_total_deadline_indefinitely() {
        let now = Instant::now();
        let mut state = Handshake::default();
        state.begin(2, now);
        for second in [10, 20, 30, 40] {
            assert!(state.advance(2, Stage::NetworkEnable, now + Duration::from_secs(second)));
        }
        assert_eq!(
            state.timeout(2, now + Duration::from_secs(45)),
            Some("host_handshake_timed_out")
        );
    }
    #[test]
    fn old_generation_and_finished_attempt_cannot_advance_or_timeout_new_attempt() {
        let now = Instant::now();
        let mut state = Handshake::default();
        state.begin(1, now);
        state.begin(3, now);
        assert!(!state.advance(1, Stage::DebuggerEnable, now));
        assert_eq!(state.timeout(1, now + Duration::from_secs(60)), None);
        assert!(state.current(3));
        state.pending = None;
        assert!(!state.advance(3, Stage::DebuggerEnable, now));
        assert_eq!(state.timeout(3, now + Duration::from_secs(60)), None);
    }
    #[test]
    fn paused_or_failed_ack_cannot_publish_ready_and_failure_keeps_window_open() {
        use super::super::types::{now_ms, Control};
        use std::sync::{
            atomic::{AtomicBool, AtomicU64},
            Arc, Mutex,
        };
        let events = Arc::new(Mutex::new(Vec::<HostEvent>::new()));
        let output = events.clone();
        let handle = HostHandle {
            label: "browser-research-test".into(),
            control: Arc::new(Control {
                active: AtomicBool::new(true),
                generation: AtomicU64::new(1),
                closed: AtomicBool::new(false),
                expires_at_ms: now_ms() + 60000,
                handshake: Mutex::default(),
                sink: Arc::new(move |event| output.lock().unwrap().push(event)),
            }),
        };
        let first = handle.begin_handshake(false);
        handle.pause();
        assert!(!handle.finish_handshake(first));
        let next = handle.begin_handshake(true);
        assert!(handle.handshake_current(next));
        handle.fail_handshake(first, "old_failure");
        assert!(handle.active());
        handle.fail_handshake(next, "page_enable_timed_out");
        assert!(!handle.active());
        assert!(!handle.control.closed.load(Ordering::SeqCst));
        assert!(!handle.handshake_stage(next, Stage::DebuggerEnable));
        assert!(!handle.finish_handshake(next));
        let events = events.lock().unwrap();
        assert_eq!(
            events.iter().filter(|event| event.kind == "failed").count(),
            1
        );
        assert!(!events.iter().any(|event| event.kind == "ready"));
        let failed = events.last().unwrap();
        assert_eq!(failed.generation, next);
        assert!(failed.url.is_empty() && failed.body.is_none() && failed.request_body.is_none());
        drop(events);
        let attempt = handle.begin_handshake(true);
        handle.control.generation.fetch_add(1, Ordering::SeqCst);
        handle.navigation_during_handshake();
        assert!(!handle.active());
        assert!(!handle.finish_handshake(attempt));
        assert!(!handle.control.closed.load(Ordering::SeqCst));
    }
}
