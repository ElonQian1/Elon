use serde_json::{json, Value};
use std::collections::BTreeMap;

const HOST_HEARTBEAT_TTL_MS: u128 = 20_000;

pub(super) fn heartbeat_live(value: Option<u128>, now: u128) -> bool {
    value.is_some_and(|value| now.saturating_sub(value) <= HOST_HEARTBEAT_TTL_MS)
}

pub(super) fn validate_release_identity(value: &str) -> Result<String, String> {
    let (version, git_sha) = value
        .rsplit_once('+')
        .ok_or_else(|| "target_release_identity 必须是 version+git_sha。".to_string())?;
    let version_ok = !version.is_empty()
        && version.len() <= 48
        && version
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'));
    let git_sha_ok =
        (40..=64).contains(&git_sha.len()) && git_sha.bytes().all(|byte| byte.is_ascii_hexdigit());
    if !version_ok || !git_sha_ok {
        return Err("target_release_identity 不是合法的精确 Win 发布身份。".to_string());
    }
    Ok(format!("{}+{}", version, git_sha.to_ascii_lowercase()))
}

#[derive(Default)]
pub(super) struct DesktopRelease {
    processes: BTreeMap<u32, (String, u128)>,
    unknown_at: Option<u128>,
}

impl DesktopRelease {
    pub(super) fn ready(
        &self,
        node: &str,
        target: &str,
        frontend_at: Option<u128>,
        now: u128,
    ) -> bool {
        validate_release_identity(node).ok().as_deref() == Some(target)
            && self.identity(now) == Some(target)
            && heartbeat_live(frontend_at, now)
    }
    pub(super) fn observe(&mut self, kind: &str, fields: &Value, now: u128) {
        // Native event history is replayed after reconnect; it is not a live proof.
        if kind != "bridge.heartbeat" || fields.get("native_seq").is_some() {
            return;
        }
        self.processes
            .retain(|_, (_, at)| now.saturating_sub(*at) <= HOST_HEARTBEAT_TTL_MS);
        let identity = fields["desktop_release_identity"]
            .as_str()
            .and_then(|value| validate_release_identity(value).ok());
        let pid = fields["desktop_process_id"]
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value > 0);
        if let (Some(identity), Some(pid)) = (identity, pid) {
            if self.processes.len() < 8 || self.processes.contains_key(&pid) {
                self.processes.insert(pid, (identity, now));
                return;
            }
        }
        self.unknown_at = Some(now);
    }

    pub(super) fn identity(&self, now: u128) -> Option<&str> {
        if heartbeat_live(self.unknown_at, now) {
            return None;
        }
        let mut live = self
            .processes
            .values()
            .filter(|(_, at)| heartbeat_live(Some(*at), now));
        let (identity, _) = live.next()?;
        live.all(|(other, _)| other == identity)
            .then_some(identity.as_str())
    }

    pub(super) fn snapshot(&self, now: u128) -> Value {
        json!({
            "release_identity": self.identity(now),
            "process_ids": self.processes.iter()
                .filter(|(_, (_, at))| heartbeat_live(Some(*at), now))
                .map(|(pid, _)| *pid).collect::<Vec<_>>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_node_alone_is_not_a_ready_desktop() {
        let mut desktop = DesktopRelease::default();
        let target = format!("0.3.69+{}", "a".repeat(40));
        assert!(!desktop.ready(&target, &target, Some(100), 100));
        desktop.observe(
            "bridge.heartbeat",
            &json!({"desktop_process_id": 123, "desktop_release_identity": target}),
            100,
        );
        assert!(desktop.ready(&target, &target, Some(100), 100));
        assert!(!desktop.ready(&target, &target, None, 100));
        assert!(!desktop.ready(&target, &target, Some(100), 30_000));
        assert!(!desktop.ready("old", &target, Some(100), 100));
        assert_eq!(desktop.snapshot(100)["process_ids"], json!([123]));
        assert!(validate_release_identity("latest").is_err());
    }

    #[test]
    fn only_fresh_unambiguous_process_heartbeats_prove_a_desktop_release() {
        let mut desktop = DesktopRelease::default();
        let a = format!("0.3.69+{}", "a".repeat(40));
        let b = format!("0.3.69+{}", "b".repeat(40));
        let first = json!({"desktop_process_id": 1, "desktop_release_identity": a});
        desktop.observe("action.receipt", &first, 100);
        assert_eq!(desktop.identity(100), None);
        desktop.observe("bridge.heartbeat", &first, 100);
        assert_eq!(desktop.identity(100), Some(a.as_str()));
        desktop.observe(
            "bridge.heartbeat",
            &json!({"native_seq": 1, "desktop_process_id": 2, "desktop_release_identity": b}),
            100,
        );
        assert_eq!(desktop.identity(100), Some(a.as_str()));
        desktop.observe(
            "bridge.heartbeat",
            &json!({"desktop_process_id": 2, "desktop_release_identity": b}),
            100,
        );
        assert_eq!(desktop.identity(100), None);
        assert_eq!(desktop.identity(100 + HOST_HEARTBEAT_TTL_MS + 1), None);
        desktop.observe("bridge.heartbeat", &first, 30_000);
        assert_eq!(desktop.identity(30_000), Some(a.as_str()));
        desktop.observe("bridge.heartbeat", &json!({}), 30_000);
        assert_eq!(desktop.identity(30_000), None);
    }
}
