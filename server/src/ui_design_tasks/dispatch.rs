use crate::{
    project_ws_protocol::ProjectAttachmentRef,
    store::{Store, UiLearnedRoute},
};

use super::{
    force_ui_design_task, intent::classify_ui_route, intent::UiRouteClass, UiDesignTaskInput,
};

pub(crate) struct ResolvedUiRouteTask {
    pub(crate) task: Option<UiDesignTaskInput>,
    pub(crate) suppress_inference: bool,
    pub(crate) source: &'static str,
}

pub(crate) fn resolve_ui_route_task(
    store: &Store,
    project_id: &str,
    display_message: &str,
    explicit_task: Option<&UiDesignTaskInput>,
    attachments: Option<&[ProjectAttachmentRef]>,
) -> ResolvedUiRouteTask {
    if let Some(task) = explicit_task {
        return ResolvedUiRouteTask {
            task: Some(task.clone()),
            suppress_inference: false,
            source: "explicit",
        };
    }
    if let Ok(Some(entry)) = store.lookup_ui_route_learning(project_id, display_message) {
        let clustered = entry.match_kind.as_deref() == Some("controlled_cluster");
        return match entry.learned_route {
            UiLearnedRoute::Ui => {
                let mut task = force_ui_design_task(display_message, attachments);
                task.route_learning_id = Some(entry.id);
                task.route_learning_origin = Some(if clustered {
                    "active_cluster".to_string()
                } else {
                    "active_library".to_string()
                });
                task.route_learning_phrase = Some(display_message.chars().take(2_000).collect());
                ResolvedUiRouteTask {
                    task: Some(task),
                    suppress_inference: false,
                    source: if clustered {
                        "active_cluster_ui"
                    } else {
                        "active_library_ui"
                    },
                }
            }
            UiLearnedRoute::NonUi => ResolvedUiRouteTask {
                task: None,
                suppress_inference: true,
                source: if clustered {
                    "active_cluster_non_ui"
                } else {
                    "active_library_non_ui"
                },
            },
        };
    }
    if store
        .has_ui_route_cluster_conflict(project_id, display_message)
        .unwrap_or(false)
    {
        let mut task = force_ui_design_task(display_message, attachments);
        task.route_learning_origin = Some("cluster_conflict".to_string());
        task.route_learning_phrase = Some(display_message.chars().take(2_000).collect());
        return ResolvedUiRouteTask {
            task: Some(task),
            suppress_inference: false,
            source: "cluster_conflict_secondary_rescue",
        };
    }

    let decision = classify_ui_route(display_message, attachments);
    if decision.class == UiRouteClass::Ambiguous {
        let mut task = force_ui_design_task(display_message, attachments);
        task.route_learning_origin = Some("ambiguous_local".to_string());
        task.route_learning_phrase = Some(display_message.chars().take(2_000).collect());
        return ResolvedUiRouteTask {
            task: Some(task),
            suppress_inference: false,
            source: "ambiguous_secondary_rescue",
        };
    }
    if decision.class == UiRouteClass::ConfirmedUi {
        let mut task = force_ui_design_task(display_message, attachments);
        task.route_learning_origin = Some("local_confirmed".to_string());
        task.route_learning_phrase = Some(display_message.chars().take(2_000).collect());
        return ResolvedUiRouteTask {
            task: Some(task),
            suppress_inference: false,
            source: "local_confirmed_ui",
        };
    }
    ResolvedUiRouteTask {
        task: None,
        suppress_inference: false,
        source: "local_confirmed_non_ui",
    }
}

pub(crate) fn promote_codex_ui_route(message: &str) -> Result<String, String> {
    let mut task = force_ui_design_task(message, None);
    task.route_learning_origin = Some("codex_rescue".to_string());
    task.route_learning_phrase = Some(message.chars().take(2_000).collect());
    super::append_ui_design_task_context(message.to_string(), Some(&task), None, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::UiRouteLearningSource;

    fn store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon-ui-route-dispatch-{}.db",
            uuid::Uuid::new_v4().simple()
        ));
        Store::open(&path).unwrap()
    }

    #[test]
    fn ambiguous_phrase_is_promoted_to_secondary_rescue_task() {
        let resolved = resolve_ui_route_task(
            &store(),
            "project-1",
            "让底部轻一点，看起来更克制",
            None,
            None,
        );
        assert_eq!(resolved.source, "ambiguous_secondary_rescue");
        assert_eq!(
            resolved.task.unwrap().route_learning_origin.as_deref(),
            Some("ambiguous_local")
        );
    }

    #[test]
    fn active_non_ui_experience_blocks_keyword_reclassification() {
        let store = store();
        store
            .confirm_ui_route_learning(
                "project-1",
                Some("user-1"),
                "调整按钮点击逻辑",
                UiLearnedRoute::NonUi,
                UiRouteLearningSource::UserOverride,
                "explicit correction",
            )
            .unwrap();
        let resolved =
            resolve_ui_route_task(&store, "project-1", "请帮我调整按钮点击逻辑", None, None);
        assert!(resolved.suppress_inference);
        assert!(resolved.task.is_none());
    }

    #[test]
    fn codex_rescue_adds_a_trusted_ui_contract_and_learning_phrase() {
        let promoted = promote_codex_ui_route("让操作区更有呼吸感").unwrap();
        assert!(promoted.contains("<elon-ui-design-task version=\"1\">"));
        assert!(promoted.contains("\"route_learning_origin\":\"codex_rescue\""));
        assert!(promoted.contains("ui_confirm_route"));
    }

    #[test]
    fn controlled_synonym_reuses_verified_route_without_secondary_rescue() {
        let store = store();
        store
            .confirm_ui_route_learning(
                "project-1",
                Some("user-1"),
                "按钮太胖",
                UiLearnedRoute::Ui,
                UiRouteLearningSource::UserOverride,
                "explicit correction",
            )
            .unwrap();
        let resolved = resolve_ui_route_task(&store, "project-1", "主操作太厚重", None, None);
        assert_eq!(resolved.source, "active_cluster_ui");
        assert_eq!(
            resolved.task.unwrap().route_learning_origin.as_deref(),
            Some("active_cluster")
        );
    }

    #[test]
    fn conflicting_cluster_forces_secondary_confirmation() {
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
        let resolved = resolve_ui_route_task(&store, "project-1", "主操作太厚重", None, None);
        assert_eq!(resolved.source, "cluster_conflict_secondary_rescue");
        assert_eq!(
            resolved.task.unwrap().route_learning_origin.as_deref(),
            Some("cluster_conflict")
        );
    }
}
