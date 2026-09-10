//! V1 bytes shared with sdk/game-access and the game's independent Rust verifier.
mod model;
mod verify;
mod wire;
pub use model::*;
pub use verify::{verify_observation, verify_observation_json};
pub use wire::{authorization_digest, challenge_digest, observation_message};

pub(super) fn required_scope(action: &Action) -> &str {
    action.scope()
}
