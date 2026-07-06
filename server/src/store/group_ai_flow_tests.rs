    use super::*;
    use crate::group_ai::types::{
        CreateMatterRecord, CreateMergeRequestInput, RecordAssignmentArtifactInput,
        RecordReviewInput,
    };
    use serde_json::json;

    fn temp_store() -> Store {
        let path = std::env::temp_dir().join(format!(
            "elon-group-ai-flow-test-{}.sqlite",
            uuid::Uuid::new_v4().simple()
        ));
        let _ = std::fs::remove_file(&path);
        Store::open(&path).expect("store should open")
    }

    #[test]
    fn assignment_status_and_events_are_durable() {
        let store = temp_store();
        let user = store
            .create_user("group-ai-flow@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Group AI Flow", None, None)
            .expect("project should be created")
            .project;
        let channel = store
            .list_project_space_channels(&user.id, &project.id)
            .expect("channels should list")
            .into_iter()
            .find(|channel| channel.kind == "ai_development")
            .expect("ai development channel should exist");
        let matter = store
            .create_project_ai_matter(CreateMatterRecord {
                project_id: project.id.clone(),
                channel_id: channel.id,
                requester_user_id: user.id.clone(),
                source_message_id: None,
                title: "多 AI 协作".to_string(),
                brief: "验证 Matter assignment 状态流".to_string(),
                collaboration_mode: "critic".to_string(),
                participant_user_ids: vec![user.id.clone()],
                node_policy_json: json!({ "mode": "project_write" }),
                acceptance_criteria: vec!["事件可追踪".to_string()],
                plan_json: json!({ "roles": [] }),
            })
            .expect("matter should be created");
        let assignment = store
            .create_project_ai_matter_assignment(CreateMatterAssignmentRecord {
                matter_id: matter.id.clone(),
                bot_id: "bot:codex".to_string(),
                assignee_user_id: Some(user.id.clone()),
                provider_user_id: user.id.clone(),
                node_id: "node-a".to_string(),
                role: "critic".to_string(),
                runtime_route: "pc_node_cli".to_string(),
                cli_name: "codex".to_string(),
                worktree_path: None,
                branch_name: Some("group-ai/test".to_string()),
                status: "planned".to_string(),
            })
            .expect("assignment should be created");

        let updated = store
            .update_project_ai_matter_assignment_status(
                &assignment.id,
                "failed",
                Some("节点离线，等待重试"),
            )
            .expect("assignment should update");
        assert_eq!(updated.status, "failed");
        assert_eq!(
            updated.result_summary.as_deref(),
            Some("节点离线，等待重试")
        );

        let executed = store
            .update_project_ai_matter_assignment_execution(
                &assignment.id,
                "completed",
                Some("产物已生成"),
                Some("D:/repo/.worktrees/group-ai"),
                Some("group-ai/paim-demo"),
            )
            .expect("execution artifact should update");
        assert_eq!(executed.status, "completed");
        assert_eq!(
            executed.worktree_path.as_deref(),
            Some("D:/repo/.worktrees/group-ai")
        );
        assert_eq!(executed.branch_name.as_deref(), Some("group-ai/paim-demo"));

        store
            .insert_project_ai_event(
                &project.id,
                &matter.id,
                Some(&user.id),
                "assignment_failed",
                json!({ "assignment_id": assignment.id }),
            )
            .expect("event should insert");
        let events = store
            .list_project_ai_matter_events(&project.id, &matter.id)
            .expect("events should list");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "assignment_failed");
        assert_eq!(events[0].payload["assignment_id"], assignment.id);
    }

    #[test]
    fn matter_node_policy_updates_are_durable() {
        let store = temp_store();
        let user = store
            .create_user("group-ai-policy@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Group AI Policy", None, None)
            .expect("project should be created")
            .project;
        let channel = store
            .list_project_space_channels(&user.id, &project.id)
            .expect("channels should list")
            .into_iter()
            .find(|channel| channel.kind == "ai_development")
            .expect("ai development channel should exist");
        let matter = store
            .create_project_ai_matter(CreateMatterRecord {
                project_id: project.id.clone(),
                channel_id: channel.id,
                requester_user_id: user.id.clone(),
                source_message_id: None,
                title: "预算策略".to_string(),
                brief: "验证 Matter 预算策略保存".to_string(),
                collaboration_mode: "critic".to_string(),
                participant_user_ids: vec![user.id.clone()],
                node_policy_json: json!({ "mode": "project_write" }),
                acceptance_criteria: vec!["预算可保存".to_string()],
                plan_json: json!({ "roles": [] }),
            })
            .expect("matter should be created");

        let updated = store
            .update_project_ai_matter_node_policy(
                &project.id,
                &matter.id,
                json!({
                    "mode": "project_write",
                    "budget": {
                        "max_billed_cost_rmb_fen": 120,
                        "pause_on_budget_exceeded": true
                    }
                }),
            )
            .expect("policy should update");

        assert_eq!(
            updated.node_policy["budget"]["max_billed_cost_rmb_fen"],
            120
        );
        assert_eq!(
            updated.node_policy["budget"]["pause_on_budget_exceeded"],
            true
        );
    }

    #[test]
    fn governance_artifacts_reviews_and_merge_queue_are_durable() {
        let store = temp_store();
        let user = store
            .create_user("group-ai-governance@example.com", "secret1", None, None)
            .expect("user should be created");
        let project = store
            .create_project(&user.id, "Group AI Governance", None, None)
            .expect("project should be created")
            .project;
        let channel = store
            .list_project_space_channels(&user.id, &project.id)
            .expect("channels should list")
            .into_iter()
            .find(|channel| channel.kind == "ai_development")
            .expect("ai development channel should exist");
        let matter = store
            .create_project_ai_matter(CreateMatterRecord {
                project_id: project.id.clone(),
                channel_id: channel.id,
                requester_user_id: user.id.clone(),
                source_message_id: None,
                title: "治理闭环".to_string(),
                brief: "验证产物、Review 和人工合并队列".to_string(),
                collaboration_mode: "split".to_string(),
                participant_user_ids: vec![user.id.clone()],
                node_policy_json: json!({ "mode": "project_write" }),
                acceptance_criteria: vec!["产物可追踪".to_string()],
                plan_json: json!({ "roles": [] }),
            })
            .expect("matter should be created");
        let assignment = store
            .create_project_ai_matter_assignment(CreateMatterAssignmentRecord {
                matter_id: matter.id.clone(),
                bot_id: "bot:codex".to_string(),
                assignee_user_id: Some(user.id.clone()),
                provider_user_id: user.id.clone(),
                node_id: "node-a".to_string(),
                role: "implementation".to_string(),
                runtime_route: "pc_node_cli".to_string(),
                cli_name: "codex".to_string(),
                worktree_path: Some("D:/repo/.worktrees/group-ai".to_string()),
                branch_name: Some("group-ai/impl".to_string()),
                status: "completed".to_string(),
            })
            .expect("assignment should be created");

        let artifact = store
            .record_project_ai_assignment_artifact(RecordAssignmentArtifactInput {
                project_id: project.id.clone(),
                matter_id: matter.id.clone(),
                assignment_id: assignment.id.clone(),
                uploader_user_id: Some(user.id.clone()),
                artifact_kind: "pc_node_execution_report".to_string(),
                summary: Some("节点产物已生成".to_string()),
                worktree_path: assignment.worktree_path.clone(),
                branch_name: assignment.branch_name.clone(),
                files: vec!["server/src/group_ai/api.rs".to_string()],
                diff_stat: vec!["1 file changed".to_string()],
                test_results: vec!["cargo test group_ai".to_string()],
                metadata: json!({ "schema": "project_ai.assignment_artifact.v1" }),
            })
            .expect("artifact should save");
        assert_eq!(artifact.assignment_id, assignment.id);

        let review = store
            .record_project_ai_review(RecordReviewInput {
                matter_id: matter.id.clone(),
                reviewer_bot_id: Some("bot:reviewer".to_string()),
                reviewer_user_id: Some(user.id.clone()),
                target_assignment_id: Some(assignment.id.clone()),
                severity: "medium".to_string(),
                finding: json!({
                    "schema": "project_ai.review_result.v1",
                    "status": "passed",
                    "summary": "Review 通过"
                }),
                status: "passed".to_string(),
            })
            .expect("review should save");
        assert_eq!(
            review.target_assignment_id.as_deref(),
            Some(assignment.id.as_str())
        );

        let merge_request = store
            .create_project_ai_merge_request(CreateMergeRequestInput {
                project_id: project.id.clone(),
                matter_id: matter.id.clone(),
                assignment_id: assignment.id.clone(),
                requested_by_user_id: Some(user.id.clone()),
                worktree_path: assignment.worktree_path.clone(),
                branch_name: assignment.branch_name.clone(),
                merge_strategy: "manual".to_string(),
                review_status: "passed".to_string(),
                risk_level: "medium".to_string(),
                notes: Some("人工确认后合并".to_string()),
            })
            .expect("merge request should save");
        assert_eq!(merge_request.status, "open");

        let updated = store
            .update_project_ai_merge_request(
                &project.id,
                &matter.id,
                &merge_request.id,
                crate::group_ai::types::UpdateMergeRequestRequest {
                    status: Some("merged".to_string()),
                    review_status: None,
                    risk_level: None,
                    notes: None,
                },
            )
            .expect("merge request should update");
        assert_eq!(updated.status, "merged");

        let artifacts = store
            .list_project_ai_assignment_artifacts(&project.id, &matter.id, &assignment.id)
            .expect("artifacts should list");
        let reviews = store
            .list_project_ai_reviews(&matter.id)
            .expect("reviews should list");
        let merge_requests = store
            .list_project_ai_merge_requests(&project.id, &matter.id)
            .expect("merge requests should list");
        assert_eq!(artifacts.len(), 1);
        assert_eq!(reviews.len(), 1);
        assert_eq!(merge_requests.len(), 1);
        assert_eq!(merge_requests[0].status, "merged");
    }
