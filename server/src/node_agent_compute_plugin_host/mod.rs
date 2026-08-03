//! Internal compute plugin host boundary for the Windows node.
//!
//! The first version only wraps the existing local LLM path. It does not change the cloud wire
//! protocol, publish a new capability, download plugins or claim sidecar isolation.

mod contract;
mod host;
pub(crate) mod identity;
pub(crate) mod install_plan;
mod legacy_llm;
pub(crate) mod lifecycle;
pub(crate) mod plugin_manifest;
pub(crate) mod ready_capability;

pub(crate) use contract::{ComputePluginTask, LlmChatTask};
pub(crate) use host::ComputePluginHost;
