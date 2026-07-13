//! Durable, bounded Cancel-before-Prompt fencing for the local node runtime.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process,
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use super::{now_ms, remove_file_if_exists, write_registry_temp_file, TaskJournal};
use crate::node_agent_task_journal_lock::with_task_journal_io_lock;

const CANCEL_TOMBSTONE_TTL_MS: u128 = 24 * 60 * 60 * 1_000;
const MAX_CANCEL_TOMBSTONES: usize = 4_096;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct CancelTombstoneState {
    #[serde(default)]
    generation: u64,
    #[serde(default)]
    entries: BTreeMap<String, CancelTombstone>,
    #[serde(default)]
    overflow_until_ms: Option<u128>,
    #[serde(default)]
    overflow_cooldown_until_ms: Option<u128>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CancelTombstone {
    recorded_at_ms: u128,
    expires_at_ms: u128,
}

impl TaskJournal {
    /// Persist cancellation before consulting the in-memory task table. This
    /// closes Cancel-before-Prompt across reconnects and process restarts.
    pub(crate) fn record_prestart_cancel_tombstone(&self, req_id: &str) -> Result<()> {
        validate_req_id(req_id)?;
        with_task_journal_io_lock(|| {
            let now = now_ms();
            let expires_at_ms = now.saturating_add(CANCEL_TOMBSTONE_TTL_MS);
            let mut state = self.load_cancel_tombstone_state()?;
            state.prune(now);
            if state.entries.contains_key(req_id) || state.entries.len() < MAX_CANCEL_TOMBSTONES {
                state.entries.insert(
                    req_id.to_string(),
                    CancelTombstone {
                        recorded_at_ms: now,
                        expires_at_ms,
                    },
                );
            } else if state.overflow_until_ms.is_some()
                || state.overflow_cooldown_until_ms.is_some()
            {
                // Do not keep moving a wildcard deadline forward. Cancels
                // received during the wildcard are retained exactly; if that
                // bounded follow-up registry is also exhausted, close the
                // session instead of turning overload into a permanent local
                // execution outage.
                bail!("取消墓碑容量在 overflow 冷却窗口内耗尽");
            } else {
                // A full registry must not silently forget a Cancel. A bounded
                // wildcard fence is safer than admitting a revoked prompt.
                // The equal-length cooldown prevents consecutive wildcard
                // windows from becoming one indefinite denial window.
                state.entries.clear();
                state.overflow_until_ms = Some(expires_at_ms);
                state.overflow_cooldown_until_ms =
                    Some(expires_at_ms.saturating_add(CANCEL_TOMBSTONE_TTL_MS));
            }
            self.save_cancel_tombstone_state(&state)
        })
    }

    pub(crate) fn prestart_cancel_tombstone_active(&self, req_id: &str) -> Result<bool> {
        validate_req_id(req_id)?;
        with_task_journal_io_lock(|| {
            let now = now_ms();
            let mut state = self.load_cancel_tombstone_state()?;
            let changed = state.prune(now);
            let active = state.overflow_until_ms.is_some()
                || state
                    .entries
                    .get(req_id)
                    .is_some_and(|entry| entry.expires_at_ms > now);
            if changed {
                self.save_cancel_tombstone_state(&state)?;
            }
            Ok(active)
        })
    }

    fn load_cancel_tombstone_state(&self) -> Result<CancelTombstoneState> {
        let candidates = [
            (3_u8, self.cancel_tombstones_path()),
            (2_u8, self.cancel_tombstones_previous_path()),
            (1_u8, self.cancel_tombstones_backup_path()),
        ];
        let mut first_error = None;
        let mut newest = None;
        for (priority, path) in candidates {
            if !path.exists() {
                continue;
            }
            match load_cancel_tombstone_file(&path) {
                Ok(state) => {
                    let replace =
                        newest
                            .as_ref()
                            .is_none_or(|(generation, current_priority, _)| {
                                state.generation > *generation
                                    || (state.generation == *generation
                                        && priority > *current_priority)
                            });
                    if replace {
                        newest = Some((state.generation, priority, state));
                    }
                }
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }
        if let Some((_, _, state)) = newest {
            return Ok(state);
        }
        match first_error {
            Some(error) => Err(error).context("主文件及备份取消墓碑均不可读"),
            None => Ok(CancelTombstoneState::default()),
        }
    }

    fn save_cancel_tombstone_state(&self, state: &CancelTombstoneState) -> Result<()> {
        self.ensure_dir()?;
        let path = self.cancel_tombstones_path();
        let backup_path = self.cancel_tombstones_backup_path();
        let previous_path = self.cancel_tombstones_previous_path();
        let temp_path = self.cancel_tombstones_temp_path();
        let backup_temp_path = self.cancel_tombstones_backup_temp_path();
        let previous_temp_path = self.cancel_tombstones_previous_temp_path();
        let mut next_state = state.clone();
        next_state.generation = state
            .generation
            .checked_add(1)
            .context("取消墓碑快照代次已耗尽")?;
        let bytes = serde_json::to_vec_pretty(&next_state)?;
        let main_was_valid = path.exists() && load_cancel_tombstone_file(&path).is_ok();

        // Never replace the only valid old generation first. If main is valid,
        // install a secondary copy before replacing it. If recovery came from
        // previous/backup, install main first while that recovery copy remains.
        if main_was_valid {
            self.install_cancel_tombstone_secondary(
                &backup_path,
                &backup_temp_path,
                &previous_path,
                &previous_temp_path,
                &bytes,
            )?;
            install_cancel_tombstone_snapshot(&path, &temp_path, &bytes)
                .context("安装当前代取消墓碑主快照")?;
        } else {
            install_cancel_tombstone_snapshot(&path, &temp_path, &bytes)
                .context("恢复并安装当前代取消墓碑主快照")?;
            self.install_cancel_tombstone_secondary(
                &backup_path,
                &backup_temp_path,
                &previous_path,
                &previous_temp_path,
                &bytes,
            )?;
        }
        Ok(())
    }

    fn install_cancel_tombstone_secondary(
        &self,
        backup_path: &Path,
        backup_temp_path: &Path,
        previous_path: &Path,
        previous_temp_path: &Path,
        bytes: &[u8],
    ) -> Result<()> {
        match install_cancel_tombstone_snapshot(backup_path, backup_temp_path, bytes) {
            Ok(()) => Ok(()),
            Err(error) => {
                tracing::warn!(
                    backup_path = %backup_path.display(),
                    %error,
                    "cancel tombstone backup update failed; using previous slot for current generation"
                );
                install_cancel_tombstone_snapshot(previous_path, previous_temp_path, bytes)
                    .context("安装当前代取消墓碑恢复副本")
            }
        }
    }

    fn cancel_tombstones_path(&self) -> PathBuf {
        self.dir.join("cancel-tombstones.json")
    }

    fn cancel_tombstones_backup_path(&self) -> PathBuf {
        self.dir.join("cancel-tombstones.json.bak")
    }

    fn cancel_tombstones_previous_path(&self) -> PathBuf {
        self.dir.join("cancel-tombstones.json.previous")
    }

    fn cancel_tombstones_temp_path(&self) -> PathBuf {
        self.dir
            .join(format!("cancel-tombstones.json.tmp-{}", process::id()))
    }

    fn cancel_tombstones_backup_temp_path(&self) -> PathBuf {
        self.dir
            .join(format!("cancel-tombstones.json.bak.tmp-{}", process::id()))
    }

    fn cancel_tombstones_previous_temp_path(&self) -> PathBuf {
        self.dir.join(format!(
            "cancel-tombstones.json.previous.tmp-{}",
            process::id()
        ))
    }
}

impl CancelTombstoneState {
    fn prune(&mut self, now: u128) -> bool {
        let before = self.entries.len();
        self.entries
            .retain(|_, tombstone| tombstone.expires_at_ms > now);
        let mut changed = self.entries.len() != before;
        if let Some(overflow_until_ms) = self.overflow_until_ms {
            let minimum_cooldown = overflow_until_ms.saturating_add(CANCEL_TOMBSTONE_TTL_MS);
            if self
                .overflow_cooldown_until_ms
                .is_none_or(|cooldown_until_ms| cooldown_until_ms < minimum_cooldown)
            {
                // Upgrade legacy snapshots that predate the cooldown field.
                self.overflow_cooldown_until_ms = Some(minimum_cooldown);
                changed = true;
            }
        }
        if self
            .overflow_until_ms
            .is_some_and(|overflow_until_ms| overflow_until_ms <= now)
        {
            self.overflow_until_ms = None;
            changed = true;
        }
        if self
            .overflow_cooldown_until_ms
            .is_some_and(|cooldown_until_ms| cooldown_until_ms <= now)
        {
            self.overflow_cooldown_until_ms = None;
            changed = true;
        }
        changed
    }
}

fn validate_req_id(req_id: &str) -> Result<()> {
    if req_id.is_empty() || req_id.len() > 200 || req_id.chars().any(char::is_control) {
        bail!("取消墓碑 req_id 无效");
    }
    Ok(())
}

fn load_cancel_tombstone_file(path: &Path) -> Result<CancelTombstoneState> {
    let bytes = fs::read(path).with_context(|| format!("读取 {:?}", path))?;
    serde_json::from_slice(&bytes).with_context(|| format!("解析 {:?}", path))
}

fn install_cancel_tombstone_snapshot(path: &Path, temp_path: &Path, bytes: &[u8]) -> Result<()> {
    write_registry_temp_file(temp_path, bytes)?;
    remove_file_if_exists(path)?;
    fs::rename(temp_path, path)
        .with_context(|| format!("替换取消墓碑快照 {:?} -> {:?}", temp_path, path))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(1);

    fn temp_journal(name: &str) -> (TaskJournal, PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "elon-cancel-tombstone-{name}-{}-{}",
            process::id(),
            NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        (TaskJournal::new(&path), path)
    }

    fn state_with_entry(generation: u64, req_id: &str) -> CancelTombstoneState {
        let now = now_ms();
        let mut state = CancelTombstoneState {
            generation,
            ..CancelTombstoneState::default()
        };
        state.entries.insert(
            req_id.to_string(),
            CancelTombstone {
                recorded_at_ms: now,
                expires_at_ms: now.saturating_add(CANCEL_TOMBSTONE_TTL_MS),
            },
        );
        state
    }

    fn write_state_file(path: &Path, state: &CancelTombstoneState) {
        fs::write(path, serde_json::to_vec_pretty(state).unwrap()).unwrap();
    }

    #[test]
    fn cancel_before_prompt_persists_across_restart() {
        let (journal, path) = temp_journal("restart");
        journal
            .record_prestart_cancel_tombstone("req-cancelled")
            .unwrap();

        let restarted = TaskJournal::new(&path);
        assert!(restarted
            .prestart_cancel_tombstone_active("req-cancelled")
            .unwrap());
        assert!(!restarted
            .prestart_cancel_tombstone_active("req-other")
            .unwrap());
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn expired_tombstone_is_pruned_and_does_not_deny_prompt() {
        let (journal, path) = temp_journal("expired");
        let mut state = CancelTombstoneState::default();
        state.entries.insert(
            "req-expired".to_string(),
            CancelTombstone {
                recorded_at_ms: 1,
                expires_at_ms: 1,
            },
        );
        journal.save_cancel_tombstone_state(&state).unwrap();

        assert!(!journal
            .prestart_cancel_tombstone_active("req-expired")
            .unwrap());
        assert!(journal
            .load_cancel_tombstone_state()
            .unwrap()
            .entries
            .is_empty());
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn capacity_overflow_uses_bounded_fail_closed_fence() {
        let (journal, path) = temp_journal("capacity");
        let now = now_ms();
        let mut state = CancelTombstoneState::default();
        for index in 0..MAX_CANCEL_TOMBSTONES {
            state.entries.insert(
                format!("req-{index}"),
                CancelTombstone {
                    recorded_at_ms: now,
                    expires_at_ms: now + CANCEL_TOMBSTONE_TTL_MS,
                },
            );
        }
        journal.save_cancel_tombstone_state(&state).unwrap();
        journal
            .record_prestart_cancel_tombstone("req-overflow")
            .unwrap();

        let persisted = journal.load_cancel_tombstone_state().unwrap();
        assert!(persisted.entries.is_empty());
        assert!(persisted.overflow_until_ms.is_some());
        assert!(persisted.overflow_cooldown_until_ms.is_some());
        assert!(journal
            .prestart_cancel_tombstone_active("any-valid-request")
            .unwrap());
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn overflow_deadline_is_fixed_and_later_cancels_remain_exact() {
        let (journal, path) = temp_journal("fixed-overflow");
        let now = now_ms();
        let mut full = CancelTombstoneState::default();
        for index in 0..MAX_CANCEL_TOMBSTONES {
            full.entries.insert(
                format!("req-{index}"),
                CancelTombstone {
                    recorded_at_ms: now,
                    expires_at_ms: now.saturating_add(CANCEL_TOMBSTONE_TTL_MS),
                },
            );
        }
        journal.save_cancel_tombstone_state(&full).unwrap();
        journal
            .record_prestart_cancel_tombstone("req-overflow")
            .unwrap();
        let overflow_deadline = journal
            .load_cancel_tombstone_state()
            .unwrap()
            .overflow_until_ms
            .unwrap();

        journal
            .record_prestart_cancel_tombstone("req-during-overflow")
            .unwrap();
        let mut persisted = journal.load_cancel_tombstone_state().unwrap();
        assert_eq!(persisted.overflow_until_ms, Some(overflow_deadline));
        assert!(persisted.entries.contains_key("req-during-overflow"));

        persisted.overflow_until_ms = Some(1);
        journal.save_cancel_tombstone_state(&persisted).unwrap();
        assert!(journal
            .prestart_cancel_tombstone_active("req-during-overflow")
            .unwrap());
        assert!(!journal
            .prestart_cancel_tombstone_active("unrelated-after-overflow")
            .unwrap());

        let mut cooldown_state = journal.load_cancel_tombstone_state().unwrap();
        cooldown_state.entries.clear();
        cooldown_state.overflow_until_ms = None;
        cooldown_state.overflow_cooldown_until_ms =
            Some(now_ms().saturating_add(CANCEL_TOMBSTONE_TTL_MS));
        for index in 0..MAX_CANCEL_TOMBSTONES {
            cooldown_state.entries.insert(
                format!("cooldown-{index}"),
                CancelTombstone {
                    recorded_at_ms: now,
                    expires_at_ms: now.saturating_add(CANCEL_TOMBSTONE_TTL_MS),
                },
            );
        }
        journal
            .save_cancel_tombstone_state(&cooldown_state)
            .unwrap();
        let error = journal
            .record_prestart_cancel_tombstone("would-renew-wildcard")
            .unwrap_err();
        assert!(error.to_string().contains("overflow 冷却窗口内耗尽"));
        assert!(journal
            .load_cancel_tombstone_state()
            .unwrap()
            .overflow_until_ms
            .is_none());
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn recovery_prefers_newer_previous_over_stale_backup() {
        let (journal, path) = temp_journal("recovery-generation");
        fs::create_dir_all(&path).unwrap();
        fs::write(journal.cancel_tombstones_path(), b"{broken").unwrap();
        write_state_file(
            &journal.cancel_tombstones_backup_path(),
            &state_with_entry(7, "req-stale-backup"),
        );
        write_state_file(
            &journal.cancel_tombstones_previous_path(),
            &state_with_entry(8, "req-newer-previous"),
        );

        let recovered = journal.load_cancel_tombstone_state().unwrap();
        assert_eq!(recovered.generation, 8);
        assert!(recovered.entries.contains_key("req-newer-previous"));
        assert!(!recovered.entries.contains_key("req-stale-backup"));
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn backup_failure_preserves_confirmed_generation_for_recovery() {
        let (journal, path) = temp_journal("backup-fallback");
        journal
            .save_cancel_tombstone_state(&state_with_entry(0, "req-old"))
            .unwrap();
        fs::remove_file(journal.cancel_tombstones_backup_path()).unwrap();
        fs::create_dir(journal.cancel_tombstones_backup_path()).unwrap();

        let mut current = journal.load_cancel_tombstone_state().unwrap();
        current.entries.insert(
            "req-confirmed".to_string(),
            CancelTombstone {
                recorded_at_ms: now_ms(),
                expires_at_ms: now_ms().saturating_add(CANCEL_TOMBSTONE_TTL_MS),
            },
        );
        journal.save_cancel_tombstone_state(&current).unwrap();
        fs::write(journal.cancel_tombstones_path(), b"{broken").unwrap();

        let recovered = journal.load_cancel_tombstone_state().unwrap();
        assert_eq!(recovered.generation, current.generation + 1);
        assert!(recovered.entries.contains_key("req-confirmed"));
        let _ = fs::remove_dir_all(path);
    }
}
