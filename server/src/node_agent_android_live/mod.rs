//! Android 真机 Live UI 调试数据平面。
//!
//! 浏览器控制接口受本机管理令牌保护；Android Runtime 只通过一次性会话令牌
//! 连接公开的 loopback WebSocket 路由。

mod adb_session;
mod broker;
mod protocol;
mod routes;

#[cfg(test)]
mod tests;

pub(crate) use broker::LiveUiBroker;
pub(crate) use routes::{protected_routes, runtime_routes};
