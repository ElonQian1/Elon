//! 手机、PC 工作台与远程节点共享的 UI 设计任务协议。
//!
//! 本模块只描述任务意图和确定性路由，不负责执行 Codex、构建或 Live Runtime。

mod intent;
mod model;
mod prompt;
mod dispatch;
mod learning;

pub(crate) use intent::force_ui_design_task;
pub(crate) use dispatch::resolve_ui_route_task;
pub(crate) use learning::finalize_ui_route_learning;
pub(crate) use model::{
    UiDesignAttachmentIntent, UiDesignExecutionPolicy, UiDesignRenderTarget,
    UiDesignRenderTargetKind, UiDesignTaskEvidence, UiDesignTaskInput, UiDesignTaskMode,
};
pub(crate) use prompt::{append_ui_design_task_context, ui_design_image_attachment_urls};

#[cfg(test)]
mod tests;
