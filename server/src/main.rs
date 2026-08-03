// server/src/main.rs

use anyhow::Result;
use dotenvy::dotenv;
use std::{net::SocketAddr, sync::Arc};
use tracing::info;

mod admin;
mod admin_html;
mod admin_quota;
mod admin_token_stats;
mod agent;
mod agent_api_loop;
mod agent_balloon;
mod agent_config;
mod agent_fallback;
mod agent_intent;
mod agent_llm_call;
mod agent_llm_tools;
mod agent_pc_workspace;
mod agent_prompts;
mod agent_routing;
mod agent_runtime_error_summary;
mod agent_tool_calls;
mod ai_cli;
mod ai_resource_control;
mod ai_resource_control_migration;
mod api;
mod app_update;
mod auth_api;
mod billing;
mod billing_admin;
mod billing_api;
mod billing_events;
mod billing_lifecycle;
mod billing_monitor;
mod billing_pay;
mod billing_trial_credit_migration;
mod billing_usage_source_migration;
mod chat_attachments;
mod cli_config;
mod cli_usage;
mod codex_health;
mod codex_stream;
mod codex_vault_api;
mod codex_vault_emergency_api;
mod codex_vault_emergency_migration;
mod codex_vault_slot_migration;
mod compute_usage;
mod context_compiler;
mod conversation_forks;
mod conversation_router;
mod erp_blueprint;
mod erp_blueprint_api;
mod erp_blueprint_evolution_migration;
#[cfg(test)]
mod erp_blueprint_evolution_tests;
#[cfg(test)]
mod erp_blueprint_hardening_tests;
mod erp_blueprint_mcp;
mod erp_blueprint_mcp_tools;
mod erp_blueprint_migration;
#[cfg(test)]
mod erp_blueprint_tests;
#[cfg(test)]
mod erp_blueprint_workflow_tests;
mod erp_instance_onboarding_migration;
mod errors;
mod external_app_api;
mod external_app_chat_bootstrap;
mod external_app_context;
mod external_app_context_answer_policy;
mod external_app_context_budget;
mod external_app_context_chat_evidence;
mod external_app_context_config;
mod external_app_context_contract;
mod external_app_context_example;
mod external_app_context_feedback;
mod external_app_context_gap_notice;
mod external_app_context_health;
mod external_app_context_index_contract;
mod external_app_context_observability;
mod external_app_context_pack_template;
mod external_app_context_projection;
mod external_app_context_projection_layer;
mod external_app_context_quality;
mod external_app_context_query_intent;
mod external_app_context_readiness;
mod external_app_context_response;
mod external_app_context_scenario_prompt;
mod external_app_context_source_validation;
mod external_app_context_tool_audit;
mod external_app_context_tool_execution;
mod external_app_context_tool_planner;
mod external_app_context_tool_prompt;
mod external_app_context_tool_result;
mod external_app_context_tool_result_contract;
mod external_app_context_tool_runtime;
mod external_app_context_tools;
mod external_app_http_client;
mod external_app_mvp_chat;
mod external_app_registry;
mod external_app_route_c_sdk;
mod external_app_tool_manifest;
mod external_app_tool_report_api;
mod external_app_tool_report_contract;
mod external_app_usage_policy;
mod friend_api;
mod friend_events;
mod git_command_error;
mod global_ws;
mod group_ai;
mod group_chat_project_docs;
mod group_chat_retrieval_api;
mod group_summary_api;
mod group_summary_context_pack;
mod group_summary_topic_split;
mod home_ai_search;
mod home_ai_tools;
mod home_ai_weather;
mod homecli_agent;
mod homecli_agent_project_git_worktree;
mod image_generation;
mod intent_router;
mod join_request_events;
mod lan_peer;
mod lm_chat;
mod message_recall_migration;
mod mobile_pwa_template;
mod node_agent_atomic_file;
mod node_agent_cli_security;
mod node_agent_downloads;
mod node_agent_supervision_worktree_lease;
mod node_api;
mod node_compute_admin;
mod node_compute_reservation_migration;
mod node_compute_sharing;
mod node_compute_sharing_migration;
mod node_exec_api;
mod node_hardware_probe;
mod node_install_id_migration;
mod node_llm_stream;
mod node_payout_admin;
mod node_public_dev_migration;
mod node_register_api;
mod node_registry;
mod node_router;
mod node_runtime;
mod node_scheduler;
mod offline_completion_migration;
mod open_commerce_action_confirmation_api;
mod open_commerce_action_confirmation_migration;
mod open_commerce_action_confirmation_model;
mod open_commerce_action_confirmation_service;
#[cfg(test)]
mod open_commerce_action_confirmation_tests;
mod open_commerce_adapter_api;
mod open_commerce_adapter_claim_api;
mod open_commerce_adapter_claim_migration;
mod open_commerce_adapter_claim_model;
mod open_commerce_adapter_claim_service;
#[cfg(test)]
mod open_commerce_adapter_claim_tests;
mod open_commerce_adapter_expiration_migration;
mod open_commerce_adapter_mcp;
mod open_commerce_adapter_migration;
mod open_commerce_adapter_model;
mod open_commerce_adapter_service;
#[cfg(test)]
mod open_commerce_adapter_tests;
mod open_commerce_api;
mod open_commerce_app_activity_health_model;
#[cfg(test)]
mod open_commerce_app_activity_health_tests;
mod open_commerce_app_block_api;
mod open_commerce_app_block_migration;
mod open_commerce_app_block_model;
mod open_commerce_app_block_service;
#[cfg(test)]
mod open_commerce_app_block_tests;
mod open_commerce_authorization_decision;
#[cfg(test)]
mod open_commerce_authorization_expiry_tests;
mod open_commerce_business_handoff_api;
mod open_commerce_business_handoff_mcp;
mod open_commerce_business_handoff_migration;
mod open_commerce_business_handoff_model;
mod open_commerce_business_handoff_service;
#[cfg(test)]
mod open_commerce_business_handoff_tests;
mod open_commerce_capability_contract_service;
#[cfg(test)]
mod open_commerce_capability_contract_tests;
mod open_commerce_capability_schema;
#[cfg(test)]
mod open_commerce_capability_schema_tests;
mod open_commerce_client_api;
mod open_commerce_client_lifecycle_api;
mod open_commerce_client_lifecycle_service;
mod open_commerce_client_migration;
#[cfg(test)]
mod open_commerce_client_service_tests;
mod open_commerce_connector_contract;
mod open_commerce_consumer;
mod open_commerce_consumer_model;
mod open_commerce_consumer_preference_api;
mod open_commerce_consumer_preference_mcp;
mod open_commerce_consumer_preference_migration;
mod open_commerce_consumer_preference_model;
mod open_commerce_consumer_preference_service;
#[cfg(test)]
mod open_commerce_consumer_preference_tests;
mod open_commerce_consumer_receipt_api;
mod open_commerce_consumer_receipt_mcp;
mod open_commerce_consumer_receipt_migration;
mod open_commerce_consumer_receipt_model;
mod open_commerce_consumer_receipt_service;
#[cfg(test)]
mod open_commerce_consumer_receipt_tests;
mod open_commerce_data_request_api;
mod open_commerce_data_request_migration;
mod open_commerce_data_request_model;
mod open_commerce_data_request_service;
#[cfg(test)]
mod open_commerce_data_request_tests;
mod open_commerce_developer_domain_migration;
mod open_commerce_developer_domain_service;
mod open_commerce_developer_event_api;
mod open_commerce_developer_event_migration;
mod open_commerce_developer_event_model;
mod open_commerce_developer_event_service;
#[cfg(test)]
mod open_commerce_developer_event_tests;
mod open_commerce_developer_manifest_api;
mod open_commerce_developer_manifest_migration;
mod open_commerce_developer_manifest_service;
mod open_commerce_developer_model;
mod open_commerce_directory_migration;
mod open_commerce_directory_model;
mod open_commerce_directory_service;
mod open_commerce_grant_budget_migration;
mod open_commerce_grant_budget_model;
mod open_commerce_grant_budget_service;
#[cfg(test)]
mod open_commerce_grant_budget_tests;
mod open_commerce_integration_migration;
mod open_commerce_integration_model;
mod open_commerce_invocation_protocol;
mod open_commerce_invocation_recovery;
#[cfg(test)]
mod open_commerce_invocation_recovery_tests;
mod open_commerce_mcp;
mod open_commerce_mcp_tools;
mod open_commerce_merchant_evidence_api;
mod open_commerce_merchant_evidence_mcp;
mod open_commerce_merchant_evidence_model;
mod open_commerce_merchant_evidence_service;
#[cfg(test)]
mod open_commerce_merchant_evidence_tests;
mod open_commerce_merchant_identity_api;
mod open_commerce_merchant_identity_migration;
mod open_commerce_merchant_identity_model;
mod open_commerce_merchant_identity_service;
mod open_commerce_migration;
mod open_commerce_model;
mod open_commerce_portability_adoption_api;
mod open_commerce_portability_adoption_migration;
mod open_commerce_portability_adoption_model;
mod open_commerce_portability_adoption_service;
mod open_commerce_portability_api;
mod open_commerce_portability_identity_match_migration;
mod open_commerce_portability_import_migration;
mod open_commerce_portability_import_model;
mod open_commerce_portability_import_service;
mod open_commerce_portability_migration;
mod open_commerce_portability_model;
mod open_commerce_portability_reauthorization_api;
mod open_commerce_portability_reauthorization_migration;
mod open_commerce_portability_reauthorization_model;
mod open_commerce_portability_reauthorization_service;
mod open_commerce_portability_service;
#[cfg(test)]
mod open_commerce_portability_tests;
mod open_commerce_portability_trust_api;
mod open_commerce_portability_trust_migration;
mod open_commerce_portability_trust_model;
mod open_commerce_portability_trust_service;
#[cfg(test)]
mod open_commerce_portability_v2_tests;
#[cfg(test)]
mod open_commerce_portability_v3_tests;
mod open_commerce_rate_limit_api;
mod open_commerce_rate_limit_migration;
mod open_commerce_rate_limit_model;
mod open_commerce_rate_limit_service;
#[cfg(test)]
mod open_commerce_rate_limit_tests;
mod open_commerce_relationship_api;
mod open_commerce_relationship_migration;
mod open_commerce_relationship_model;
mod open_commerce_relationship_renewal_migration;
#[cfg(test)]
mod open_commerce_relationship_renewal_tests;
mod open_commerce_relationship_service;
#[cfg(test)]
mod open_commerce_relationship_tests;
mod open_commerce_runtime_client;
mod open_commerce_runtime_migration;
mod open_commerce_runtime_model;
mod open_commerce_runtime_security;
mod open_commerce_runtime_service;
#[cfg(test)]
mod open_commerce_runtime_service_tests;
mod open_commerce_service;
#[cfg(test)]
mod open_commerce_service_tests;
mod open_commerce_webhook_api;
mod open_commerce_webhook_event_filter_migration;
mod open_commerce_webhook_history_migration;
mod open_commerce_webhook_lifecycle_migration;
mod open_commerce_webhook_migration;
mod open_commerce_webhook_model;
mod open_commerce_webhook_replay_migration;
mod open_commerce_webhook_security;
mod open_commerce_webhook_service;
mod open_commerce_webhook_verification;
mod open_commerce_webhook_verification_migration;
mod open_commerce_webhook_worker;
mod pc_agent_runtime_choice;
mod pc_node_capacity;
mod pc_node_display;
mod pc_relay;
mod pc_relay_cli_prompt;
mod pc_relay_client;
mod pc_workspace_git_remote;
mod pc_workspace_provisioner;
mod peer_relay;
mod presence_events;
mod project_android_device_leases_migration;
mod project_android_devices_migration;
mod project_api;
mod project_attachment_notes;
mod project_attachment_paths;
mod project_attachments;
mod project_auth;
mod project_channel_summary;
mod project_channels;
mod project_chat;
mod project_chat_executor;
mod project_chat_helpers;
mod project_chat_pc_node;
mod project_chat_reply;
mod project_chat_runner;
mod project_completion;
mod project_conversation_identity;
mod project_conversation_workspace;
mod project_default_docs;
mod project_deletion;
mod project_discussion_document_projection;
mod project_discussion_graph;
mod project_discussion_graph_history;
mod project_discussion_graph_model;
mod project_discussion_graph_review;
mod project_discussion_graph_validation;
mod project_discussion_source_chunks;
mod project_discussion_source_import;
mod project_discussion_source_normalizer;
mod project_discussion_source_registration;
mod project_docs;
mod project_docs_channel;
mod project_docs_scan;
mod project_docs_snapshot;
mod project_document_analysis_model;
mod project_document_architecture;
mod project_document_authorization;
mod project_document_federation;
mod project_document_federation_service;
mod project_document_file_operation_model;
mod project_document_files;
mod project_document_gateway;
mod project_document_git_transaction;
mod project_document_governance;
mod project_document_governance_facets;
mod project_document_governance_section_operations;
mod project_document_governance_service;
mod project_document_index;
mod project_document_issue_workflow;
mod project_document_knowledge_graph;
mod project_document_knowledge_graph_health;
mod project_document_knowledge_graph_model;
mod project_document_knowledge_graph_service;
mod project_document_knowledge_graph_templates;
mod project_document_maintenance;
mod project_document_organization_api;
mod project_document_policy;
mod project_document_quality;
mod project_document_quality_rules;
mod project_document_response;
mod project_document_scoped_health;
mod project_document_vault;
mod project_document_versioning;
mod project_downloads;
mod project_events;
mod project_execution_mode;
mod project_git;
mod project_git_worktree_audit;
mod project_git_worktree_audit_api;
mod project_git_worktree_global_audit_api;
mod project_join_requests;
mod project_keys;
mod project_landing;
mod project_landing_api;
mod project_membership;
mod project_mobile;
mod project_prewarm;
mod project_release_migration;
mod project_releases;
mod project_runtime_permission_api;
mod project_runtime_permission_migration;
mod project_space;
mod project_space_ai_progress;
mod project_space_task_control;
mod project_space_task_result;
mod project_space_task_snapshot;
mod project_space_task_watchdog;
mod project_storage;
mod project_storage_git;
mod project_store;
mod project_store_listing_migration;
mod project_task_scheduler;
mod project_tool_approval_recovery;
mod project_tool_approvals;
mod project_trace_events;
mod project_workspace_health;
mod project_workspace_health_monitor;
mod project_workspace_inspect;
pub(crate) mod project_workspace_lifecycle;
mod project_workspace_provision;
mod project_workspace_recovery;
mod project_ws_job;
mod project_ws_protocol;
mod project_ws_session;
mod read_receipt_events;
mod realtime_metrics;
mod release_batch;
mod release_claim;
mod release_manager;
mod release_publish_queue;
mod route_a_session_lease;
mod route_c_admin;
mod router;
mod server_agent_runtime;
mod server_agent_runtime_budget;
mod server_agent_runtime_guard;
mod server_agent_runtime_limits;
mod server_agent_runtime_output;
mod server_agent_runtime_policy;
mod server_agent_runtime_status;
mod server_trace;
mod social_ai;
mod social_ai_agents;
mod social_ai_attachment_context;
mod social_ai_message_reply;
mod source_hygiene;
mod speech_translate;
mod store;
mod store_migrations;
mod store_schema;
mod task_settlement;
mod task_settlement_correction_migration;
mod task_settlement_dispute_migration;
mod task_settlement_migration;
mod task_sui_correction_projection_migration;
mod task_sui_projection_migration;
mod task_title;
mod token_usage_api;
mod tools;
mod tools_apk;
mod tools_exec;
mod tools_git;
mod tools_patch;
mod types;
mod typing_events;
mod ui_design_tasks;
mod ui_route_learning_migration;
mod ui_tuner_api;
mod ui_tuner_context;
mod ui_tuner_device_host_api;
mod ui_tuner_device_lease_api;
mod ui_tuner_store_migration;
mod user_agent_probe;
mod user_agent_readiness;
mod user_agent_secrets;
mod user_api;
mod user_archive_api;
mod user_archive_profile;
mod user_memory_api;
mod user_memory_extract;
mod user_progression;
mod voice_asr_upload;
mod voice_audio_format;
mod voice_config;
mod voice_openai_realtime;
mod voice_openai_realtime_chat;
mod voice_protocol;
mod voice_pwcat;
mod voice_to_cli;
mod voice_tts_api;
mod voice_tts_catalog;
mod voice_tts_rewrite;
mod voice_tts_worker;
mod voice_whisper_local;
mod voice_whisper_rest;
mod voice_ws_realtime_chat;
mod voice_ws_transcribe;
mod voice_ws_virtual_mic;
mod web;
mod wechat_pay;
mod ws_client_transport;
mod ws_message;
mod ws_transport;

pub use types::AppState;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,elon_server=debug".into()),
        )
        .init();

    let state = Arc::new(AppState::new()?);
    codex_health::spawn_codex_network_monitor(state.clone());
    billing_lifecycle::spawn_reservation_janitor(state.clone());
    billing_monitor::spawn_reconciliation_monitor(state.clone());
    project_workspace_health_monitor::spawn_project_workspace_health_monitor(state.clone());
    project_document_maintenance::spawn_maintenance_worker();
    open_commerce_webhook_worker::spawn(state.clone());
    // 本地模式：作为 agent 连回云端，实现 APK→云端→PC 全双工中继
    pc_relay_client::spawn_if_configured();

    // 服务启动时：将上次运行中的频道 AI 任务标记为恢复中，而不是直接判失败。
    let interrupted = state.store.mark_interrupted_running_ws_tasks().unwrap_or(0);
    if interrupted > 0 {
        info!("{} 个进行中的任务因服务器重启被标记为已中断", interrupted);
    }
    let interrupted_tasks = state
        .store
        .mark_recovering_running_tasks_after_server_restart()
        .unwrap_or(0);
    if interrupted_tasks > 0 {
        info!(
            "{} 个数据库运行中频道任务因服务器重启进入恢复中",
            interrupted_tasks
        );
    }
    let interrupted_pc_runs = state
        .store
        .mark_interrupted_started_pc_agent_runs()
        .unwrap_or(0);
    if interrupted_pc_runs > 0 {
        info!(
            "{} 个 PC CLI 执行证明因服务器重启被标记为失败",
            interrupted_pc_runs
        );
    }
    node_llm_stream::recover_interrupted_runs(&state.store);
    node_llm_stream::spawn_expired_run_reconciler(state.clone());
    open_commerce_invocation_recovery::recover_interrupted_invocations(&state.store);
    open_commerce_invocation_recovery::spawn_expired_invocation_reconciler(state.clone());
    let interrupted_pc_sessions = state
        .store
        .mark_interrupted_running_project_execution_sessions()
        .unwrap_or(0);
    if interrupted_pc_sessions > 0 {
        info!(
            "{} 个 PC 项目执行会话因服务器重启被标记为失败",
            interrupted_pc_sessions
        );
    }
    ai_cli::pc_completion_replay::spawn_pending_pc_cli_completion_replay(state.clone());

    const STALE_RUNNING_TASK_TIMEOUT_SECS: u64 = 45 * 60;

    // 定期清理：长期 running 的任务自动标记为 failed。
    // PC 节点上的 Codex 发布/首次编译可能因为 cargo build、上传和服务重启超过 10 分钟；
    // 阈值需要覆盖真实发布窗口，避免发布已成功但频道任务先被标记失败。
    {
        let state_cleanup = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(120));
            loop {
                interval.tick().await;
                let active_channel_tasks = project_space::active_channel_ai_task_ids();
                match state_cleanup
                    .store
                    .mark_stale_running_tasks_with_channel_results_excluding(
                        STALE_RUNNING_TASK_TIMEOUT_SECS,
                        &active_channel_tasks,
                    ) {
                    Ok(n) if n > 0 => {
                        info!("{n} 个超时 running 任务已自动标记为 failed")
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!("stale task cleanup error: {e}"),
                }
            }
        });
    }

    let app = router::build_app(state);

    let addr: SocketAddr = std::env::var("LISTEN_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".into())
        .parse()?;

    info!("elon server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
