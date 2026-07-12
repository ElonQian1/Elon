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
        return match entry.learned_route {
            UiLearnedRoute::Ui => {
                let mut task = force_ui_design_task(display_message, attachments);
                task.route_learning_id = Some(entry.id);
                task.route_learning_origin = Some("active_library".to_string());
                ResolvedUiRouteTask {
                    task: Some(task),
                    suppress_inference: false,
                    source: "active_library_ui",
                }
            }
            UiLearnedRoute::NonUi => ResolvedUiRouteTask {
                task: None,
                suppress_inference: true,
                source: "active_library_non_ui",
            },
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
    ResolvedUiRouteTask {
        task: None,
        suppress_inference: false,
        source: match decision.class {
            UiRouteClass::ConfirmedUi => "local_confirmed_ui",
            UiRouteClass::ConfirmedNonUi => "local_confirmed_non_ui",
            UiRouteClass::Ambiguous => unreachable!(),
        },
    }
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
            resolved
                .task
                .unwrap()
                .route_learning_origin
                .as_deref(),
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
        let resolved = resolve_ui_route_task(
            &store,
            "project-1",
            "请帮我调整按钮点击逻辑",
            None,
            None,
        );
        assert!(resolved.suppress_inference);
        assert!(resolved.task.is_none());
    }
}
