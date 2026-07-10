//! Android 真机 Live UI 调试数据平面。
//!
//! 浏览器控制接口受本机管理令牌保护；Android Runtime 只通过一次性会话令牌
//! 连接公开的 loopback WebSocket 路由。

mod adb_session;
mod broker;
mod build_verify;
mod frame;
mod mcp;
mod mcp_tools;
mod preview;
mod protocol;
mod routes;
mod source_commit;
mod source_xml;
mod ui_ir;
mod visual_diff;
mod visual_solver;

#[cfg(test)]
mod mcp_tests;
#[cfg(test)]
mod source_commit_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_ir_tests;
#[cfg(test)]
mod visual_diff_tests;

pub(crate) use broker::LiveUiBroker;
pub(crate) use routes::{protected_routes, runtime_routes};
