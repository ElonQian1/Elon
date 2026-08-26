//! Linear share-none admission→Windows loader load-set authority.
//!
//! This source-only boundary corrects the owner graph and freezes pre/post-barrier custody. The
//! repository still has no producer for exact PE/DLL resolution or a queryable hard namespace
//! fence, so no successful transition or process launch becomes reachable in this slice.

#![allow(dead_code)]

mod digest;
mod failure;
mod launch_path_discovery;
mod launch_path_validation;
mod model;
mod model_debug;
mod namespace_validation;
mod pe_graph_validation;
mod policy;
mod resolution;
mod system_resolution_validation;
mod transition;
mod validation;

pub(in crate::node_agent_compute_plugin_host) use launch_path_discovery::WindowsRunnerLaunchContextPreCreateProjection;
pub(in crate::node_agent_compute_plugin_host) use model::LoaderLockedWorkAdmittedPluginSlot;
