//! Internal compute plugin host boundary for the Windows node.
//!
//! The first version only wraps the existing local LLM path. It does not change the cloud wire
//! protocol, publish a new capability, download plugins or claim sidecar isolation.

pub(crate) mod attempt_contract;
pub(crate) mod candidate_verification_contract;
mod candidate_verification_terminal_result;
mod contract;
pub(crate) mod fetch_contract;
pub(crate) mod fetch_file;
mod host;
pub(crate) mod identity;
pub(crate) mod install_plan;
pub(crate) mod install_plan_admission;
mod install_plan_admission_validation;
pub(crate) mod keyring;
pub(crate) mod keyring_validation;
mod legacy_llm;
pub(crate) mod lifecycle;
pub(crate) mod local_authority;
mod local_authority_schema;
pub(crate) mod manifest_validation;
pub(crate) mod plugin_manifest;
pub(crate) mod ready_capability;
pub(crate) mod runner_events;
mod signed_artifact_verification;
mod trusted_time;

pub(crate) use contract::{ComputePluginTask, LlmChatTask};
pub(crate) use host::ComputePluginHost;
pub(crate) use keyring::{ComputePluginControlPlaneKeyResolver, ComputePluginPublisherKeyResolver};
pub(crate) use signed_artifact_verification::ComputePluginEd25519PublicKey;
