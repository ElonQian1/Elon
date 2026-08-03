//! Android 真机 Live UI 调试数据平面。
//!
//! 浏览器控制接口受本机管理令牌保护；Android Runtime 只通过一次性会话令牌
//! 连接公开的 loopback WebSocket 路由。

mod adb_session;
mod annotation_mapping;
mod apk_identity;
mod broker;
mod build_verify;
mod build_verify_apk;
mod capability_gap;
mod capability_requirements;
mod cross_platform_verification;
mod debug_integration;
mod debug_integration_contract;
mod debug_package;
mod deployment_serialization;
mod design_bootstrap;
mod design_browser_runtime;
mod design_diff_regions;
mod design_drafts;
mod design_http;
mod design_session_store;
mod design_target_discovery;
mod design_targets;
mod design_tools;
mod desktop_task;
mod emulator_start;
mod fit_learning;
pub(crate) mod fit_run;
mod frame;
mod frame_artifact;
mod launcher_icon;
mod launcher_mask;
mod launcher_surface;
mod launcher_xml;
mod mcp;
mod mcp_fit_command;
mod mcp_fit_start;
mod mcp_runtime_preparation;
mod mcp_tool_contract;
mod mcp_tools;
mod node_selector;
mod preview;
mod protocol;
mod relational_layout_geometry;
mod routes;
mod runtime_binding;
mod source_commit;
mod source_json;
mod source_proof_identity;
mod source_xml;
mod style_codegen;
mod target_design_attachment;
mod task_completion;
mod tauri_host_runtime;
mod tauri_host_windows;
mod ui_ir;
mod verification_gate;
mod verification_workflow;
mod visual_diff;
mod visual_solver;
mod visual_solver_style_hints;
mod visual_solver_values;
mod window_insets_sequence;

#[cfg(test)]
mod debug_integration_tests;
#[cfg(test)]
mod design_diff_regions_tests;
#[cfg(test)]
mod design_targets_tests;
#[cfg(test)]
mod mcp_tests;
#[cfg(test)]
mod source_commit_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_ir_tests;
#[cfg(test)]
mod verification_gate_tests;
#[cfg(test)]
mod visual_diff_tests;
#[cfg(test)]
mod visual_solver_style_hints_tests;

pub(crate) use broker::LiveUiBroker;
pub(crate) use debug_package::{
    debug_base_package_name, fixed_node_debug_suffix, node_debug_fingerprint,
    normalize_debug_package_name, resolve_debug_application_id_suffix,
    scoped_debug_application_id_suffix,
};
pub(crate) use mcp::descriptor_for_project as mcp_descriptor_for_project;
pub(crate) use routes::{protected_routes, runtime_routes};

pub(crate) fn design_target_profile(root: &std::path::Path) -> anyhow::Result<serde_json::Value> {
    let (targets, files_inspected, truncated) = design_target_discovery::discover_targets(root)?;
    Ok(serde_json::json!({
        "schemaVersion": 1,
        "targets": targets,
        "scan": {
            "filesInspected": files_inspected,
            "truncated": truncated,
            "contentEmbedded": false,
        }
    }))
}
