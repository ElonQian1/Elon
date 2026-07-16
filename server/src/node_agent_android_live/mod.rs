//! Android 真机 Live UI 调试数据平面。
//!
//! 浏览器控制接口受本机管理令牌保护；Android Runtime 只通过一次性会话令牌
//! 连接公开的 loopback WebSocket 路由。

mod adb_session;
mod annotation_mapping;
mod broker;
mod build_verify;
mod build_verify_apk;
mod capability_gap;
mod capability_requirements;
mod cross_platform_verification;
mod debug_package;
mod design_bootstrap;
mod design_diff_regions;
mod desktop_task;
mod fit_learning;
pub(crate) mod fit_run;
mod frame;
mod frame_artifact;
mod mcp;
mod mcp_tools;
mod preview;
mod protocol;
mod relational_layout_geometry;
mod routes;
mod source_commit;
mod source_json;
mod source_xml;
mod style_codegen;
mod target_design_attachment;
mod task_completion;
mod ui_ir;
mod verification_gate;
mod visual_diff;
mod visual_solver;
mod visual_solver_style_hints;
mod visual_solver_values;
mod window_insets_sequence;

#[cfg(test)]
mod design_diff_regions_tests;
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
pub(crate) use debug_package::scoped_debug_application_id_suffix;
pub(crate) use mcp::descriptor_for_project as mcp_descriptor_for_project;
pub(crate) use routes::{protected_routes, runtime_routes};
