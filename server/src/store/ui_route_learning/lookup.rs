use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use super::{
    load_entry, map_entry, normalize_ui_route_phrase, record_event, UiRouteLearningEntry,
    UiRouteLearningAlias, UiRouteLearningSource, GLOBAL_SCOPE_ID, MAX_SAMPLE_CHARS,
};
use crate::store::{common::new_id, common::now, Store};

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
        if let Some(mut entry) = query_exact(&conn, project_id, &phrase_key)? {
            let timestamp = now();
            conn.execute(
                "UPDATE ui_route_learning_entries
                 SET hit_count = hit_count + 1, last_hit_at = ?2, updated_at = ?2
                 WHERE id = ?1",
                params![entry.id, timestamp],
            )?;
            entry.match_kind = Some("exact".to_string());
            return Ok(Some(entry));
        }

        let Some(concept) = crate::ui_design_tasks::controlled_ui_route_concept(message) else {
            return Ok(None);
        };
        let entries = query_cluster(
            &conn,
            project_id,
            concept.key,
            concept.version as i64,
        )?;
        let Some(first) = entries.first() else {
            return Ok(None);
        };
        if entries
            .iter()
            .any(|entry| entry.learned_route != first.learned_route)
        {
            record_cluster_conflict(&conn, &entries, message)?;
            return Ok(None);
        }

        let timestamp = now();
        conn.execute(
            "UPDATE ui_route_learning_entries
             SET hit_count = hit_count + 1,
                 cluster_hit_count = cluster_hit_count + 1,
                 last_hit_at = ?2,
                 last_cluster_hit_at = ?2,
                 updated_at = ?2
             WHERE id = ?1",
            params![first.id, timestamp],
        )?;
        conn.execute(
            "INSERT INTO ui_route_learning_aliases (
               id, entry_id, phrase_key, sample_text, source, status,
               evidence_count, conflict_count, hit_count, last_hit_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, 'controlled_vocabulary', 'active', 1, 0, 1, ?5, ?5, ?5)
             ON CONFLICT(entry_id, phrase_key) DO UPDATE SET
               sample_text = excluded.sample_text,
               status = 'active',
               hit_count = ui_route_learning_aliases.hit_count + 1,
               last_hit_at = excluded.last_hit_at,
               updated_at = excluded.updated_at",
            params![
                new_id("ui_route_alias"),
                first.id,
                phrase_key,
                message.trim().chars().take(MAX_SAMPLE_CHARS).collect::<String>(),
                timestamp,
            ],
        )?;
        record_event(
            &conn,
            &first.id,
            "controlled_alias_matched",
            first.learned_route,
            UiRouteLearningSource::ControlledVocabulary,
            None,
            message,
        )?;
        let mut matched = load_entry(&conn, &first.id)?;
        matched.match_kind = Some("controlled_cluster".to_string());
        matched.matched_alias = Some(message.trim().chars().take(MAX_SAMPLE_CHARS).collect());
        Ok(Some(matched))
    }
}

fn query_exact(
    conn: &rusqlite::Connection,
    project_id: &str,
    phrase_key: &str,
) -> Result<Option<UiRouteLearningEntry>> {
    Ok(conn
        .query_row(
            "SELECT id, scope_type, scope_id, phrase_key, sample_text, learned_route,
                    status, source, confidence, evidence_count, conflict_count, hit_count,
                    created_by_user_id, created_at, updated_at,
                    concept_key, concept_version, cluster_hit_count,
                    (SELECT COUNT(*) FROM ui_route_learning_aliases alias
                     WHERE alias.entry_id = ui_route_learning_entries.id AND alias.status = 'active')
             FROM ui_route_learning_entries
             WHERE phrase_key = ?1 AND status = 'active'
               AND ((scope_type = 'project' AND scope_id = ?2)
                    OR (scope_type = 'global' AND scope_id = ?3))
             ORDER BY CASE scope_type WHEN 'project' THEN 0 ELSE 1 END
             LIMIT 1",
            params![phrase_key, project_id, GLOBAL_SCOPE_ID],
            map_entry,
        )
        .optional()?)
}

fn query_cluster(
    conn: &rusqlite::Connection,
    project_id: &str,
    concept_key: &str,
    concept_version: i64,
) -> Result<Vec<UiRouteLearningEntry>> {
    let mut statement = conn.prepare(
        "SELECT id, scope_type, scope_id, phrase_key, sample_text, learned_route,
                status, source, confidence, evidence_count, conflict_count, hit_count,
                created_by_user_id, created_at, updated_at,
                concept_key, concept_version, cluster_hit_count,
                (SELECT COUNT(*) FROM ui_route_learning_aliases alias
                 WHERE alias.entry_id = ui_route_learning_entries.id AND alias.status = 'active')
         FROM ui_route_learning_entries
         WHERE concept_key = ?1 AND concept_version = ?2 AND status = 'active'
           AND ((scope_type = 'project' AND scope_id = ?3)
                OR (scope_type = 'global' AND scope_id = ?4))
         ORDER BY CASE scope_type WHEN 'project' THEN 0 ELSE 1 END, updated_at DESC",
    )?;
    let entries = statement
        .query_map(
            params![concept_key, concept_version, project_id, GLOBAL_SCOPE_ID],
            map_entry,
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(entries)
}

fn record_cluster_conflict(
    conn: &rusqlite::Connection,
    entries: &[UiRouteLearningEntry],
    message: &str,
) -> Result<()> {
    let timestamp = now();
    for entry in entries {
        conn.execute(
            "UPDATE ui_route_learning_entries
             SET conflict_count = conflict_count + 1, updated_at = ?2 WHERE id = ?1",
            params![entry.id, timestamp],
        )?;
        record_event(
            conn,
            &entry.id,
            "cluster_conflict_blocked",
            entry.learned_route,
            UiRouteLearningSource::ControlledVocabulary,
            None,
            message,
        )?;
    }
    Ok(())
}

pub(super) fn load_aliases(
    conn: &rusqlite::Connection,
    entry_id: &str,
) -> Result<Vec<UiRouteLearningAlias>> {
    let mut statement = conn.prepare(
        "SELECT id, sample_text, source, status, hit_count, conflict_count, updated_at
         FROM ui_route_learning_aliases
         WHERE entry_id = ?1
         ORDER BY CASE status WHEN 'active' THEN 0 WHEN 'candidate' THEN 1 ELSE 2 END,
                  updated_at DESC",
    )?;
    let aliases = statement
        .query_map(params![entry_id], |row| {
            Ok(UiRouteLearningAlias {
                id: row.get(0)?,
                sample_text: row.get(1)?,
                source: row.get(2)?,
                status: row.get(3)?,
                hit_count: row.get(4)?,
                conflict_count: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(aliases)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::UiRouteLearningSource;
    use crate::ui_route_learning_migration::{migration_v101, migration_v97};

    fn store() -> Store {
        let connection = rusqlite::Connection::open_in_memory().unwrap();
        migration_v97(&connection).unwrap();
        migration_v101(&connection).unwrap();
        Store {
            conn: std::sync::Mutex::new(connection),
        }
    }

    #[test]
    fn verified_phrase_is_reused_by_controlled_synonyms() {
        let store = store();
        store
            .confirm_ui_route_learning(
                "project-1",
                Some("user-1"),
                "按钮太胖",
                UiLearnedRoute::Ui,
                UiRouteLearningSource::UserOverride,
                "user confirmed",
            )
            .unwrap();

        for synonym in ["按钮显得笨重", "主操作太厚重"] {
            let matched = store
                .lookup_ui_route_learning("project-1", synonym)
                .unwrap()
                .unwrap();
            assert_eq!(matched.learned_route, UiLearnedRoute::Ui);
            assert_eq!(matched.match_kind.as_deref(), Some("controlled_cluster"));
            assert_eq!(matched.matched_alias.as_deref(), Some(synonym));
        }
        let entries = store.list_ui_route_learning("project-1", 10).unwrap();
        assert_eq!(entries[0].cluster_hit_count, 2);
        assert_eq!(entries[0].alias_count, 2);
    }

    #[test]
    fn conflicting_active_routes_block_cluster_reuse() {
        let store = store();
        for (message, route) in [
            ("按钮太胖", UiLearnedRoute::Ui),
            ("按钮显得笨重", UiLearnedRoute::NonUi),
        ] {
            store
                .confirm_ui_route_learning(
                    "project-1",
                    Some("user-1"),
                    message,
                    route,
                    UiRouteLearningSource::UserOverride,
                    "explicit correction",
                )
                .unwrap();
        }
        assert!(store
            .lookup_ui_route_learning("project-1", "主操作太厚重")
            .unwrap()
            .is_none());
        assert!(store
            .list_ui_route_learning("project-1", 10)
            .unwrap()
            .iter()
            .all(|entry| entry.conflict_count > 0));
    }
}
