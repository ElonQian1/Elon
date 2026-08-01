use anyhow::{bail, Result};
use serde_json::Value;

use crate::{
    group_ai::types::{
        ProjectAiAssignmentArtifact, ProjectAiMatter, ProjectAiMatterAssignment,
        MATTER_STATUS_CANCELED, MATTER_STATUS_DONE, MATTER_STATUS_FAILED,
        MATTER_STATUS_REVIEW_READY, MATTER_STATUS_RUNNING,
    },
    store::Store,
};

use super::instance_service::ONBOARDING_EXISTING_PROJECT;
use super::model::{
    ErpBlueprint, ErpBlueprintVersion, ErpInstance, ErpMaterializationArtifactContract,
    ErpMaterializationAssignmentSummary, ErpMaterializationConfiguration,
    ErpMaterializationContract, ErpMaterializationEvidenceSummary, ErpMaterializationMatterSummary,
    ErpMaterializationSource, ErpMaterializationStatus, MATERIALIZATION_CONTRACT_SCHEMA,
    MATERIALIZATION_EVIDENCE_SCHEMA,
};

const STATUS_SCHEMA: &str = "yilong.erp.materialization_status.v1";
const ARTIFACT_KIND: &str = "erp_instance_materialization";
const INSTANCE_MANIFEST_PATH: &str = ".yilong/erp-instance.json";

pub(crate) fn build_contract(
    blueprint: &ErpBlueprint,
    version: &ErpBlueprintVersion,
    instance: &ErpInstance,
) -> ErpMaterializationContract {
    let mut verification_requirements = vec![
        "实例清单必须与当前固定版本和配置修订一致",
        "测试结果必须登记到 Assignment artifact",
        "不得把密钥、经营原始数据或其他商户私有源码写入产物",
        "合并、验收和发布必须由商户人工确认",
    ];
    let mut boundaries = vec![
        "does_not_copy_source_automatically",
        "does_not_start_assignment_automatically",
        "does_not_merge_or_publish_automatically",
        "does_not_attest_deployment_or_payment",
    ];
    if instance.onboarding_mode == ONBOARDING_EXISTING_PROJECT {
        verification_requirements.insert(
            0,
            "先盘点现有项目模块、数据迁移与私有扩展，再以最小改动补齐蓝图能力",
        );
        boundaries.push("does_not_overwrite_existing_project");
    }
    ErpMaterializationContract {
        schema: MATERIALIZATION_CONTRACT_SCHEMA,
        instance_id: instance.id.clone(),
        instance_key: instance.instance_key.clone(),
        target_project_id: instance.project_id.clone(),
        target_onboarding_mode: instance.onboarding_mode.clone(),
        source: ErpMaterializationSource {
            project_id: blueprint.definition.source_project_id.clone(),
            git_commit: version.manifest.source_git_commit.clone(),
            blueprint_key: blueprint.definition.blueprint_key.clone(),
            version: version.manifest.version.clone(),
        },
        configuration: ErpMaterializationConfiguration {
            revision: instance.configuration_revision,
            industry: instance.industry.clone(),
            theme_key: instance.theme_key.clone(),
            enabled_modules: instance.enabled_modules.clone(),
            plugins: instance.plugins.clone(),
            private_extensions: instance.private_extensions.clone(),
        },
        required_artifact: ErpMaterializationArtifactContract {
            artifact_kind: ARTIFACT_KIND,
            instance_manifest_path: INSTANCE_MANIFEST_PATH,
            evidence_schema: MATERIALIZATION_EVIDENCE_SCHEMA,
            required_metadata_fields: vec![
                "schema",
                "instance_id",
                "configuration_revision",
                "source_git_commit",
                "instance_manifest_path",
                "instance_manifest_sha256",
                "verification_passed",
            ],
        },
        verification_requirements,
        boundaries,
    }
}

pub(crate) fn status(
    store: &Store,
    project_id: &str,
    instance_id: &str,
) -> Result<ErpMaterializationStatus> {
    let instance = store.erp_instance(instance_id)?;
    if instance.project_id != project_id {
        bail!("只有商户实例所属项目可以读取物化状态");
    }
    let blueprint = store.erp_blueprint(&instance.blueprint_id)?;
    let version = store.erp_blueprint_version(&instance.pinned_version_id)?;
    let contract = build_contract(&blueprint, &version, &instance);
    let Some(matter_id) = instance.bootstrap_matter_id.as_deref() else {
        return Ok(ErpMaterializationStatus {
            schema: STATUS_SCHEMA,
            state: "not_planned".into(),
            recoverable: true,
            contract,
            matter: None,
            evidence: Vec::new(),
            blockers: vec!["尚未创建初始化 Matter".into()],
            next_action: "由商户创建初始化 Matter；系统只保存计划，不会自动执行。".into(),
        });
    };
    let matter = store
        .get_project_ai_matter(project_id, matter_id)?
        .ok_or_else(|| anyhow::anyhow!("实例关联的初始化 Matter 不存在"))?;
    let assignments = store.list_project_ai_matter_assignments(matter_id)?;
    let evidence = collect_evidence(store, &instance, &version, &assignments)?;
    let assignment_summary = summarize_assignments(&assignments);
    let contract_value = serde_json::to_value(&contract)?;
    let plan_contract_matches = matter.plan.get("execution_contract") == Some(&contract_value);
    let valid_evidence = evidence.iter().filter(|item| item.valid).count();
    let (state, mut recoverable, mut blockers, mut next_action) =
        derive_state(&matter, &assignment_summary, valid_evidence);
    if !plan_contract_matches {
        recoverable = true;
        blockers.push("Matter 计划未携带当前配置修订的物化合同".into());
        next_action = if state == "executing" || state == "awaiting_acceptance" {
            "先完成或取消旧 Matter；正在执行或等待验收的计划不能被新配置覆盖。".into()
        } else {
            "由商户再次确认初始化 Matter，系统会保留旧 Matter 并为当前配置修订生成新计划。".into()
        };
    }
    Ok(ErpMaterializationStatus {
        schema: STATUS_SCHEMA,
        state,
        recoverable,
        contract,
        matter: Some(ErpMaterializationMatterSummary {
            id: matter.id,
            status: matter.status,
            decision: matter.final_decision,
            plan_contract_matches,
            assignments: assignment_summary,
        }),
        evidence,
        blockers,
        next_action,
    })
}

fn collect_evidence(
    store: &Store,
    instance: &ErpInstance,
    version: &ErpBlueprintVersion,
    assignments: &[ProjectAiMatterAssignment],
) -> Result<Vec<ErpMaterializationEvidenceSummary>> {
    let mut result = Vec::new();
    for assignment in assignments {
        for artifact in store.list_project_ai_assignment_artifacts(
            &instance.project_id,
            &assignment.matter_id,
            &assignment.id,
        )? {
            if artifact.artifact_kind == ARTIFACT_KIND {
                result.push(validate_evidence(instance, version, artifact));
            }
        }
    }
    result.sort_by(|left, right| right.created_at.cmp(&left.created_at));
    Ok(result)
}

fn validate_evidence(
    instance: &ErpInstance,
    version: &ErpBlueprintVersion,
    artifact: ProjectAiAssignmentArtifact,
) -> ErpMaterializationEvidenceSummary {
    let metadata = artifact
        .metadata
        .get("erp_materialization")
        .unwrap_or(&artifact.metadata);
    let mut issues = Vec::new();
    check_string(
        metadata,
        "schema",
        MATERIALIZATION_EVIDENCE_SCHEMA,
        &mut issues,
    );
    check_string(metadata, "instance_id", &instance.id, &mut issues);
    check_i64(
        metadata,
        "configuration_revision",
        instance.configuration_revision,
        &mut issues,
    );
    check_string(
        metadata,
        "source_git_commit",
        &version.manifest.source_git_commit,
        &mut issues,
    );
    check_string(
        metadata,
        "instance_manifest_path",
        INSTANCE_MANIFEST_PATH,
        &mut issues,
    );
    let digest = metadata
        .get("instance_manifest_sha256")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if digest.len() != 64 || !digest.chars().all(|value| value.is_ascii_hexdigit()) {
        issues.push("instance_manifest_sha256 必须是 64 位十六进制摘要".into());
    }
    if metadata.get("verification_passed").and_then(Value::as_bool) != Some(true) {
        issues.push("verification_passed 必须为 true".into());
    }
    ErpMaterializationEvidenceSummary {
        artifact_id: artifact.id,
        assignment_id: artifact.assignment_id,
        valid: issues.is_empty(),
        issues,
        created_at: artifact.created_at,
    }
}

fn summarize_assignments(
    assignments: &[ProjectAiMatterAssignment],
) -> ErpMaterializationAssignmentSummary {
    let mut summary = ErpMaterializationAssignmentSummary {
        total: assignments.len(),
        planned: 0,
        running: 0,
        completed: 0,
        failed: 0,
        failed_assignment_ids: Vec::new(),
    };
    for assignment in assignments {
        match assignment.status.as_str() {
            "completed" => summary.completed += 1,
            "failed" => {
                summary.failed += 1;
                summary.failed_assignment_ids.push(assignment.id.clone());
            }
            "running" | "queued" | "dispatching" => summary.running += 1,
            _ => summary.planned += 1,
        }
    }
    summary
}

fn derive_state(
    matter: &ProjectAiMatter,
    assignments: &ErpMaterializationAssignmentSummary,
    valid_evidence: usize,
) -> (String, bool, Vec<String>, String) {
    if matter.status == MATTER_STATUS_CANCELED {
        return (
            "canceled".into(),
            true,
            vec!["初始化 Matter 已取消".into()],
            "如需重新初始化，先由商户明确创建新的受控计划。".into(),
        );
    }
    if matter.status == MATTER_STATUS_FAILED || assignments.failed > 0 {
        return (
            "execution_failed".into(),
            true,
            vec![format!("{} 个 Assignment 执行失败", assignments.failed)],
            "检查失败产物和节点日志，修复后使用现有 Assignment 重试。".into(),
        );
    }
    if matter.status == MATTER_STATUS_DONE {
        return if valid_evidence > 0 {
            (
                "accepted_verified".into(),
                false,
                Vec::new(),
                "初始化已由人工验收；发布仍按项目发布门禁单独进行。".into(),
            )
        } else {
            (
                "accepted_without_manifest_evidence".into(),
                true,
                vec!["Matter 已验收，但没有匹配当前实例修订的物化证据".into()],
                "补录实例清单摘要和验证结果，不能把人工验收等同于已发布。".into(),
            )
        };
    }
    let all_completed = assignments.total > 0 && assignments.completed == assignments.total;
    if matter.status == MATTER_STATUS_REVIEW_READY || all_completed {
        return if valid_evidence > 0 {
            (
                "awaiting_acceptance".into(),
                true,
                Vec::new(),
                "由商户审核产物、合并队列和验证证据后决定是否验收。".into(),
            )
        } else {
            (
                "awaiting_materialization_evidence".into(),
                true,
                vec!["Assignment 已完成，但尚无有效 ERP 物化证据".into()],
                "按合同登记实例清单摘要、测试结果和配置修订，再进入人工验收。".into(),
            )
        };
    }
    if matter.status == MATTER_STATUS_RUNNING || assignments.running > 0 || assignments.total > 0 {
        return (
            "executing".into(),
            true,
            Vec::new(),
            "等待 Assignment 完成；失败时从原 Assignment 重试，不新建重复 Matter。".into(),
        );
    }
    let role_count = matter
        .plan
        .get("roles")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if role_count == 0 {
        return (
            "blocked_no_authorized_bot".into(),
            true,
            vec!["创建 Matter 时没有可用的项目授权 Bot".into()],
            "先为项目授权在线节点和 CLI，再由商户重新确认执行计划。".into(),
        );
    }
    if matter.final_decision.as_deref() == Some("approved") {
        (
            "ready_to_start".into(),
            true,
            Vec::new(),
            "由商户显式启动 Matter，系统随后才创建 Assignment。".into(),
        )
    } else {
        (
            "awaiting_approval".into(),
            true,
            Vec::new(),
            "商户先检查物化合同、预算和 Bot，再批准并启动 Matter。".into(),
        )
    }
}

fn check_string(metadata: &Value, key: &str, expected: &str, issues: &mut Vec<String>) {
    if metadata.get(key).and_then(Value::as_str) != Some(expected) {
        issues.push(format!("{key} 与当前实例合同不一致"));
    }
}

fn check_i64(metadata: &Value, key: &str, expected: i64, issues: &mut Vec<String>) {
    if metadata.get(key).and_then(Value::as_i64) != Some(expected) {
        issues.push(format!("{key} 与当前实例合同不一致"));
    }
}
