use super::context_projection;
use crate::{
    project_feature_registry::{ProjectFeaturePriority, ProjectFeatureStatus},
    project_feature_registry_service::{
        register_feature, transition_feature, RegisterFeatureRequest,
    },
    project_feature_registry_store::load_registry,
};
use std::{fs, path::PathBuf};

struct ProjectionFixture(PathBuf);

impl ProjectionFixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "elon_project_feature_projection_{}",
            uuid::Uuid::new_v4().simple()
        ));
        fs::create_dir_all(path.join("docs")).unwrap();
        let status = crate::git_command_error::git_command()
            .args(["init", "--quiet"])
            .current_dir(&path)
            .status()
            .unwrap();
        assert!(status.success());
        Self(path)
    }

    fn add_ready(&self, id: &str, priority: ProjectFeaturePriority) {
        let requirement_path = format!("docs/{id}.md");
        fs::write(
            self.0.join(&requirement_path),
            format!("---\nversion_status: current\n---\n# {id}\n\nPRIVATE_BODY_{id}\n"),
        )
        .unwrap();
        let expected_registry_revision = self
            .0
            .join(".elon/project-features.json")
            .exists()
            .then(|| load_registry(&self.0).unwrap().revision)
            .flatten();
        let registered = register_feature(
            &self.0,
            RegisterFeatureRequest {
                id: id.to_string(),
                title: format!("Feature {id}"),
                summary: format!("Implement the shared context projection for {id}."),
                status: ProjectFeatureStatus::Proposed,
                priority,
                requirement_path,
                knowledge_node_id: String::new(),
                owner: "codex".to_string(),
                tags: vec!["projection".to_string()],
                task_paths: vec!["src/shared.rs".to_string()],
                dependencies: Vec::new(),
                acceptance_criteria: vec!["Projection is bounded.".to_string()],
                actor: "codex-test".to_string(),
                reason: "projection contract".to_string(),
                expected_registry_revision,
            },
        )
        .unwrap();
        let accepted = transition_feature(
            &self.0,
            id,
            ProjectFeatureStatus::Accepted,
            "codex-test",
            "accept projection fixture",
            "",
            registered["registry_revision"].as_str(),
        )
        .unwrap();
        transition_feature(
            &self.0,
            id,
            ProjectFeatureStatus::Ready,
            "codex-test",
            "ready projection fixture",
            "",
            accepted["registry_revision"].as_str(),
        )
        .unwrap();
    }
}

impl Drop for ProjectionFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn projection_skips_drifted_candidates_and_fills_the_three_item_budget() {
    let fixture = ProjectionFixture::new();
    fixture.add_ready("drifted", ProjectFeaturePriority::P0);
    for id in ["alpha", "beta", "gamma", "delta"] {
        fixture.add_ready(id, ProjectFeaturePriority::P1);
    }
    fs::write(
        fixture.0.join("docs/drifted.md"),
        "---\nversion_status: current\n---\n# changed scope\nPRIVATE_DRIFTED_BODY\n",
    )
    .unwrap();

    let projection = context_projection(
        &fixture.0,
        "下一步继续完善功能需求",
        &["src/shared.rs".to_string()],
    );
    assert_eq!(projection["status"], "ok");
    assert_eq!(projection["candidate_count"], 5);
    assert_eq!(projection["selected_count"], 3);
    assert_eq!(projection["invalidated_count"], 1);
    assert_eq!(projection["invalidated"][0]["id"], "drifted");
    assert_eq!(
        projection["invalidated"][0]["reason"],
        "requirement_drifted"
    );
    assert!(projection["selected"]
        .as_array()
        .unwrap()
        .iter()
        .all(|feature| feature["id"] != "drifted"));
    assert_eq!(projection["source_bodies_returned"], 0);
    let serialized = serde_json::to_string(&projection).unwrap();
    assert!(!serialized.contains("PRIVATE_BODY"));
    assert!(!serialized.contains("PRIVATE_DRIFTED_BODY"));
}
