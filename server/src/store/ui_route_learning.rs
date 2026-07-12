use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension, Row};
use serde::Serialize;

use super::{common::new_id, common::now, Store};

const GLOBAL_SCOPE_ID: &str = "*";
const MAX_SAMPLE_CHARS: usize = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UiLearnedRoute {
    Ui,
    NonUi,
}

impl UiLearnedRoute {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ui => "ui",
            Self::NonUi => "non_ui",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value {
            "ui" => Ok(Self::Ui),
            "non_ui" => Ok(Self::NonUi),
            _ => Err(anyhow!("未知 UI 路由经验值: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiRouteLearningSource {
    CodexProposal,
    UserOverride,
    RuntimeVerified,
    Admin,
}

impl UiRouteLearningSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::CodexProposal => "codex_proposal",
            Self::UserOverride => "user_override",
            Self::RuntimeVerified => "runtime_verified",
            Self::Admin => "admin",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiRouteLearningEntry {
    pub(crate) id: String,
    pub(crate) scope_type: String,
    pub(crate) scope_id: String,
    pub(crate) phrase_key: String,
    pub(crate) sample_text: String,
    pub(crate) learned_route: UiLearnedRoute,
    pub(crate) status: String,
    pub(crate) source: String,
    pub(crate) confidence: f64,
    pub(crate) evidence_count: i64,
    pub(crate) conflict_count: i64,
    pub(crate) hit_count: i64,
    pub(crate) created_by_user_id: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl Store {
    pub(crate) fn lookup_ui_route_learning(
        &self,
        project_id: &str,
        message: &str,
    ) -> Result<Option<UiRouteLearningEntry>> {
        let phrase_key = normalize_ui_route_phrase(message);
        if phrase_key.is_empty() {
            return Ok(None);
        }
        let conn = self.conn()?;
        let entry = conn
            .query_row(
                "SELECT id, scope_type, scope_id, phrase_key, sample_text, learned_route,
                        status, source, confidence, evidence_count, conflict_count, hit_count,
                        created_by_user_id, created_at, updated_at
                 FROM ui_route_learning_entries
                 WHERE phrase_key = ?1 AND status = 'active'
                   AND ((scope_type = 'project' AND scope_id = ?2)
                        OR (scope_type = 'global' AND scope_id = ?3))
                 ORDER BY CASE scope_type WHEN 'project' THEN 0 ELSE 1 END
                 LIMIT 1",
                params![phrase_key, project_id, GLOBAL_SCOPE_ID],
                map_entry,
            )
            .optional()?;
        if let Some(entry) = entry.as_ref() {
            let timestamp = now();
            conn.execute(
                "UPDATE ui_route_learning_entries
                 SET hit_count = hit_count + 1, last_hit_at = ?2, updated_at = ?2
                 WHERE id = ?1",
                params![entry.id, timestamp],
            )?;
        }
        Ok(entry)
    }

    pub(crate) fn record_ui_route_candidate(
        &self,
        project_id: &str,
        user_id: Option<&str>,
        message: &str,
        learned_route: UiLearnedRoute,
        confidence: f64,
        evidence: &str,
    ) -> Result<UiRouteLearningEntry> {
        self.upsert_ui_route_learning(
            project_id,
            user_id,
            message,
            learned_route,
            UiRouteLearningSource::CodexProposal,
            confidence,
            evidence,
            false,
        )
    }

    pub(crate) fn confirm_ui_route_learning(
        &self,
        project_id: &str,
        user_id: Option<&str>,
        message: &str,
        learned_route: UiLearnedRoute,
        source: UiRouteLearningSource,
        evidence: &str,
    ) -> Result<UiRouteLearningEntry> {
        if source == UiRouteLearningSource::CodexProposal {
            return Err(anyhow!("模型建议不能直接晋升为稳定 UI 路由经验"));
        }
        self.upsert_ui_route_learning(
            project_id,
            user_id,
            message,
            learned_route,
            source,
            1.0,
            evidence,
            true,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn upsert_ui_route_learning(
        &self,
        project_id: &str,
        user_id: Option<&str>,
        message: &str,
        learned_route: UiLearnedRoute,
        source: UiRouteLearningSource,
        confidence: f64,
        evidence: &str,
        activate: bool,
    ) -> Result<UiRouteLearningEntry> {
        let phrase_key = normalize_ui_route_phrase(message);
        if phrase_key.chars().count() < 2 {
            return Err(anyhow!("UI 路由经验文本过短"));
        }
        let sample_text = message.trim().chars().take(MAX_SAMPLE_CHARS).collect::<String>();
        let timestamp = now();
        let conn = self.conn()?;
        let existing = conn
            .query_row(
                "SELECT id, learned_route, status FROM ui_route_learning_entries
                 WHERE scope_type = 'project' AND scope_id = ?1 AND phrase_key = ?2",
                params![project_id, phrase_key],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?;
        let entry_id = existing
            .as_ref()
            .map(|item| item.0.clone())
            .unwrap_or_else(|| new_id("ui_route"));
        let conflict = existing
            .as_ref()
            .is_some_and(|item| item.1 != learned_route.as_str());
        let status = if activate { "active" } else { "candidate" };
        if existing.is_some() {
            conn.execute(
                "UPDATE ui_route_learning_entries
                 SET sample_text = ?2,
                     learned_route = CASE WHEN ?3 = 1 THEN ?4 ELSE learned_route END,
                     status = CASE WHEN ?3 = 1 THEN 'active' ELSE status END,
                     source = CASE WHEN ?3 = 1 THEN ?5 ELSE source END,
                     confidence = MAX(confidence, ?6),
                     evidence_count = evidence_count + CASE WHEN ?7 = 0 THEN 1 ELSE 0 END,
                     conflict_count = conflict_count + ?7,
                     created_by_user_id = COALESCE(?8, created_by_user_id),
                     updated_at = ?9
                 WHERE id = ?1",
                params![
                    entry_id,
                    sample_text,
                    activate as i64,
                    learned_route.as_str(),
                    source.as_str(),
                    confidence.clamp(0.0, 1.0),
                    conflict as i64,
                    user_id,
                    timestamp,
                ],
            )?;
        } else {
            conn.execute(
                "INSERT INTO ui_route_learning_entries (
                   id, scope_type, scope_id, phrase_key, sample_text, learned_route, status,
                   source, confidence, evidence_count, conflict_count, hit_count,
                   created_by_user_id, created_at, updated_at
                 ) VALUES (?1, 'project', ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, 0, 0, ?9, ?10, ?10)",
                params![
                    entry_id,
                    project_id,
                    phrase_key,
                    sample_text,
                    learned_route.as_str(),
                    status,
                    source.as_str(),
                    confidence.clamp(0.0, 1.0),
                    user_id,
                    timestamp,
                ],
            )?;
        }
        record_event(
            &conn,
            &entry_id,
            if conflict { "conflict" } else if activate { "confirmed" } else { "proposed" },
            learned_route,
            source,
            user_id,
            evidence,
        )?;
        load_entry(&conn, &entry_id)
    }

    pub(crate) fn revoke_ui_route_learning(
        &self,
        project_id: &str,
        entry_id: &str,
        user_id: &str,
        reason: &str,
    ) -> Result<UiRouteLearningEntry> {
        let timestamp = now();
        let conn = self.conn()?;
        let changed = conn.execute(
            "UPDATE ui_route_learning_entries SET status = 'revoked', updated_at = ?3
             WHERE id = ?1 AND scope_type = 'project' AND scope_id = ?2",
            params![entry_id, project_id, timestamp],
        )?;
        if changed == 0 {
            return Err(anyhow!("UI 路由经验不存在"));
        }
        let entry = load_entry(&conn, entry_id)?;
        record_event(
            &conn,
            entry_id,
            "revoked",
            entry.learned_route,
            UiRouteLearningSource::UserOverride,
            Some(user_id),
            reason,
        )?;
        Ok(entry)
    }

    pub(crate) fn list_ui_route_learning(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<UiRouteLearningEntry>> {
        let conn = self.conn()?;
        let mut statement = conn.prepare(
            "SELECT id, scope_type, scope_id, phrase_key, sample_text, learned_route,
                    status, source, confidence, evidence_count, conflict_count, hit_count,
                    created_by_user_id, created_at, updated_at
             FROM ui_route_learning_entries
             WHERE scope_type = 'project' AND scope_id = ?1
             ORDER BY updated_at DESC LIMIT ?2",
        )?;
        Ok(statement
            .query_map(params![project_id, limit.clamp(1, 200) as i64], map_entry)?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }
}

pub(crate) fn normalize_ui_route_phrase(message: &str) -> String {
    let mut normalized = message.to_lowercase();
    for filler in ["麻烦", "请你", "请帮我", "帮我", "帮忙", "一下", "可以吗", "好吗"] {
        normalized = normalized.replace(filler, "");
    }
    normalized
        .chars()
        .filter(|value| value.is_alphanumeric() || is_cjk(*value))
        .take(512)
        .collect()
}

fn is_cjk(value: char) -> bool {
    matches!(value as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF)
}

fn map_entry(row: &Row<'_>) -> rusqlite::Result<UiRouteLearningEntry> {
    let learned_route = row.get::<_, String>(5)?;
    Ok(UiRouteLearningEntry {
        id: row.get(0)?,
        scope_type: row.get(1)?,
        scope_id: row.get(2)?,
        phrase_key: row.get(3)?,
        sample_text: row.get(4)?,
        learned_route: UiLearnedRoute::parse(&learned_route).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                5,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        status: row.get(6)?,
        source: row.get(7)?,
        confidence: row.get(8)?,
        evidence_count: row.get(9)?,
        conflict_count: row.get(10)?,
        hit_count: row.get(11)?,
        created_by_user_id: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn load_entry(conn: &rusqlite::Connection, entry_id: &str) -> Result<UiRouteLearningEntry> {
    Ok(conn.query_row(
        "SELECT id, scope_type, scope_id, phrase_key, sample_text, learned_route,
                status, source, confidence, evidence_count, conflict_count, hit_count,
                created_by_user_id, created_at, updated_at
         FROM ui_route_learning_entries WHERE id = ?1",
        params![entry_id],
        map_entry,
    )?)
}

fn record_event(
    conn: &rusqlite::Connection,
    entry_id: &str,
    action: &str,
    learned_route: UiLearnedRoute,
    source: UiRouteLearningSource,
    user_id: Option<&str>,
    evidence: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO ui_route_learning_events (
           id, entry_id, action, learned_route, source, actor_user_id, evidence, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            new_id("ui_route_event"),
            entry_id,
            action,
            learned_route.as_str(),
            source.as_str(),
            user_id,
            evidence.chars().take(2_000).collect::<String>(),
            now(),
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_route_learning_migration::migration_v97;

    fn store() -> Store {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        migration_v97(&connection).unwrap();
        Store {
            conn: std::sync::Mutex::new(connection),
        }
    }

    #[test]
    fn codex_proposal_stays_candidate_until_trusted_confirmation() {
        let store = store();
        let candidate = store
            .record_ui_route_candidate(
                "project-1",
                Some("user-1"),
                "让底部轻一点",
                UiLearnedRoute::Ui,
                0.82,
                "secondary classifier",
            )
            .unwrap();
        assert_eq!(candidate.status, "candidate");
        assert!(store
            .lookup_ui_route_learning("project-1", "请帮我让底部轻一点")
            .unwrap()
            .is_none());

        let active = store
            .confirm_ui_route_learning(
                "project-1",
                Some("user-1"),
                "让底部轻一点",
                UiLearnedRoute::Ui,
                UiRouteLearningSource::RuntimeVerified,
                "ui_get_runtime_status + ui_apply_live_patch succeeded",
            )
            .unwrap();
        assert_eq!(active.status, "active");
        assert_eq!(
            store
                .lookup_ui_route_learning("project-1", "请帮我让底部轻一点")
                .unwrap()
                .unwrap()
                .learned_route,
            UiLearnedRoute::Ui
        );
    }

    #[test]
    fn explicit_user_override_can_correct_and_revoke_an_entry() {
        let store = store();
        let active = store
            .confirm_ui_route_learning(
                "project-1",
                Some("user-1"),
                "调整按钮响应速度",
                UiLearnedRoute::NonUi,
                UiRouteLearningSource::UserOverride,
                "user selected normal development",
            )
            .unwrap();
        assert_eq!(active.status, "active");
        let revoked = store
            .revoke_ui_route_learning("project-1", &active.id, "user-1", "wrong example")
            .unwrap();
        assert_eq!(revoked.status, "revoked");
        assert!(store
            .lookup_ui_route_learning("project-1", "调整按钮响应速度")
            .unwrap()
            .is_none());
    }
}
