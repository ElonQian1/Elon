use super::{history, policy, token, Fixture};
use crate::esk_asset::platform::{sellback::*, PlatformPolicy};

mod access_projection;
mod access_wire_export;
mod auth;
mod boundaries;
mod concurrency;
mod integrity;
mod lifecycle;
mod migration;
mod pagination;

fn setup() -> (Fixture, PlatformPolicy, SellbackConfiguration) {
    let fixture = Fixture::new();
    let formal = policy(1_000_000_000);
    history::post(&fixture, &formal, "alice", 0);
    history::post(&fixture, &formal, "bob", 1);
    let mut body = crate::esk_asset::platform::sellback::tests::fixture_policy(
        "alice",
        &formal.source_fingerprint,
    )
    .body;
    body.eligible_user_ids.push("bob".into());
    let config = SellbackConfiguration::Enabled(validate_policy(body).unwrap());
    (fixture, formal, config)
}

fn page(fixture: &Fixture, user: &str, config: &SellbackConfiguration) -> SellbackPage {
    fixture
        .store
        .esk_platform_sellback_page(user, &token(user), 20, None, config)
        .unwrap()
}

fn input(
    fixture: &Fixture,
    user: &str,
    key: &str,
    units: i64,
    config: &SellbackConfiguration,
) -> SellbackSubmitInput {
    let page = page(fixture, user, config);
    let policy = page.summary.availability.policy.unwrap();
    SellbackSubmitInput {
        idempotency_key: key.into(),
        amount_base_units: units,
        expected_snapshot_digest: page.summary.snapshot_digest,
        policy_digest: policy.policy_digest,
        terms_digest: policy.body.terms_digest,
    }
}

fn submit(
    fixture: &Fixture,
    user: &str,
    key: &str,
    units: i64,
    config: &SellbackConfiguration,
) -> SellbackResult {
    fixture
        .store
        .submit_esk_platform_sellback(
            user,
            &token(user),
            &input(fixture, user, key, units, config),
            config,
        )
        .unwrap()
}

fn error<T: std::fmt::Debug>(result: anyhow::Result<T>, expected: SellbackError) {
    let actual = result.unwrap_err();
    assert_eq!(
        actual.downcast_ref::<SellbackError>(),
        Some(&expected),
        "{actual:#}"
    );
}
