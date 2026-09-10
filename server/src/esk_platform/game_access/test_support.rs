use super::{
    model::{Error, Result},
    policy::{hash, Policy},
};
use serde_json::{json, Value};
use std::cell::RefCell;

thread_local! {
    static POLICY: RefCell<Option<Option<Value>>> = const { RefCell::new(None) };
}
pub(crate) struct Guard(Option<Option<Value>>);
impl Drop for Guard {
    fn drop(&mut self) {
        POLICY.with(|v| {
            v.replace(self.0.take());
        });
    }
}
pub(crate) fn service() -> String {
    format!("egs_{}", "42".repeat(32))
}
pub(crate) fn enable() -> Guard {
    let input = json!({"schema":"esk.game.access.policy.v1", "main_issuer":"synthetic-main",
        "client_id":"esk-game.web", "redirect_uri":"https://game.example.test/api/account/callback",
        "service_secret_sha256":hash(&service()),"key_id":"synthetic-key","signing_seed_hex":"44".repeat(32)});
    Guard(POLICY.with(|v| v.replace(Some(Some(input)))))
}
pub(crate) fn disabled() -> Guard {
    Guard(POLICY.with(|v| v.replace(Some(None))))
}
pub(crate) fn current_policy() -> Option<Result<Policy>> {
    POLICY.with(|v| v.borrow().clone()).map(|v| match v {
        Some(input) => Policy::from_input(serde_json::from_value(input).unwrap()),
        None => Err(Error::Disabled.into()),
    })
}
