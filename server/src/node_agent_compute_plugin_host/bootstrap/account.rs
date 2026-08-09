use super::{ComputePluginBootstrap, ComputePluginBootstrapAccountBinding};

impl ComputePluginBootstrap {
    /// Any credential replacement is a control-authority boundary, including secret/token
    /// rotation under the same node and owner. The caller invokes this before publishing the new
    /// credentials, so every intent and request derived from the prior session fails closed.
    pub(crate) fn note_credentials_replaced(&self, account: Option<(&str, &str)>) {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => {
                self.invalidate_policy_binding_intents_after_poison();
                return;
            }
        };
        state.advance_configuration_generation();
        match state.cancellation_generation.checked_add(1) {
            Some(next) => state.cancellation_generation = next,
            None => state.configuration_exhausted = true,
        }
        state.account =
            account.map(
                |(node_id, owner_user_id)| ComputePluginBootstrapAccountBinding {
                    node_id: node_id.to_string(),
                    owner_user_id: owner_user_id.to_string(),
                },
            );
        state.sharing_requested = false;
        state.desired_policy = None;
        state.authorization_high_water = None;
        state.initialization_plan = None;
        state.last_install_plan_preparation = None;
        state.last_install_plan_planning_snapshot = None;
    }
}
