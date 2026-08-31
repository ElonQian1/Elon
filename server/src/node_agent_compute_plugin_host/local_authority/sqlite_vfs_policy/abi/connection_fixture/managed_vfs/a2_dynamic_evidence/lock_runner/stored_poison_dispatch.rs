//! Completion-aware dispatch without widening the q3 wire/native-receipt contract.

use std::path::Path;

use super::{
    stored_poison, stored_poison_route_unknown, LockRunnerStoredPoisonBindingV1,
    LockRunnerStoredPoisonCompletionV1,
};

pub(super) fn validate_binding(binding: LockRunnerStoredPoisonBindingV1) -> anyhow::Result<()> {
    match binding.completion {
        LockRunnerStoredPoisonCompletionV1::RetentionSucceeded => {
            stored_poison::validate_binding(binding)
        }
        LockRunnerStoredPoisonCompletionV1::RetentionRouteUnknown => {
            stored_poison_route_unknown::validate_binding(binding)
        }
    }
}

pub(super) fn exact_selector(binding: LockRunnerStoredPoisonBindingV1) -> String {
    match binding.completion {
        LockRunnerStoredPoisonCompletionV1::RetentionSucceeded => {
            stored_poison::exact_selector(binding)
        }
        LockRunnerStoredPoisonCompletionV1::RetentionRouteUnknown => {
            stored_poison_route_unknown::exact_selector(binding)
        }
    }
}

pub(super) fn exercise_child(
    root: &Path,
    binding: LockRunnerStoredPoisonBindingV1,
) -> anyhow::Result<()> {
    match binding.completion {
        LockRunnerStoredPoisonCompletionV1::RetentionSucceeded => {
            stored_poison::exercise_child(root, binding)
        }
        LockRunnerStoredPoisonCompletionV1::RetentionRouteUnknown => {
            stored_poison_route_unknown::exercise_child(root, binding)
        }
    }
}

pub(super) fn validate_payload(
    payload: &str,
    binding: LockRunnerStoredPoisonBindingV1,
) -> anyhow::Result<stored_poison::ValidatedStoredPoisonPayloadV1> {
    match binding.completion {
        LockRunnerStoredPoisonCompletionV1::RetentionSucceeded => {
            stored_poison::validate_payload(payload, binding)
        }
        LockRunnerStoredPoisonCompletionV1::RetentionRouteUnknown => {
            stored_poison_route_unknown::validate_payload(payload, binding)
        }
    }
}
