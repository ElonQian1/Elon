//! Pure production validators; never starts a node, cloud request, or Android.
#![allow(dead_code)]
#[path = "../../../src/node_agent_android_live/debug_package.rs"]
mod debug_package;
#[path = "../../../src/node_agent_android_live/native_runtime_proof_validation.rs"]
mod native_proof;
#[path = "../../../src/node_agent_android_live/registered_project_identity/validation.rs"]
mod registered_project_identity;
use native_proof::validate as validate_native;
#[path = "../../../src/node_agent_android_live/semantic_parity/contract.rs"]
mod contract;
#[path = "../../../src/node_agent_android_live/semantic_parity/evidence.rs"]
mod evidence;
#[path = "../../../src/node_agent_android_live/semantic_parity/tests.rs"]
mod semantic_tests;
#[path = "../../../src/node_agent_android_live/semantic_parity/signing.rs"]
mod signing;
