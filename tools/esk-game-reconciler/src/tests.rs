use super::{model::*, settlement::Signed, verify::*, *};
use ed25519_dalek::{Signer, SigningKey};

pub(crate) fn sign<T: serde::Serialize>(payload: T, domain: &str, seed: u8) -> Signed<T> {
    let key = SigningKey::from_bytes(&[seed; 32]);
    let signature_hex = hex::encode(key.sign(&canonical(domain, &payload).unwrap()).to_bytes());
    Signed {
        payload,
        signature_hex,
    }
}
pub(crate) fn fixture() -> (Config, Request) {
    let config = Config {
        schema: "esk.game.reconciliation.config.v1".into(),
        policy_digest: "11".repeat(32),
        source_id: "independent-ledger".into(),
        source_public_key_hex: hex::encode(
            SigningKey::from_bytes(&[31; 32]).verifying_key().as_bytes(),
        ),
        reconciler_public_key_hex: hex::encode(
            SigningKey::from_bytes(&[32; 32]).verifying_key().as_bytes(),
        ),
        account_scope: "participant-custody-7".into(),
        user_id: "user-7".into(),
        beneficiary: format!("0x{}", "22".repeat(32)),
        wallet_binding_digest: "33".repeat(32),
        asset_type: format!("0x{}::cash::USD", "44".repeat(32)),
        asset_decimals: 6,
        accounting_started_at_ms: "100".into(),
    };
    let statement = Statement {
        schema: "esk.game.reconciliation.source.v1".into(),
        source_id: config.source_id.clone(),
        environment: Environment::Live,
        basis: Basis::ParticipantCumulativeCashV1,
        policy_digest: config.policy_digest.clone(),
        account_scope: config.account_scope.clone(),
        user_id: config.user_id.clone(),
        beneficiary: config.beneficiary.clone(),
        wallet_binding_digest: config.wallet_binding_digest.clone(),
        asset_type: config.asset_type.clone(),
        asset_decimals: 6,
        accounting_started_at_ms: "100".into(),
        sequence: "1".into(),
        previous_source_digest: "0".repeat(64),
        period_start_ms: "100".into(),
        period_end_ms: "200".into(),
        cumulative_realized_trading_pnl_units: "2100".into(),
        cumulative_trading_fees_units: "50".into(),
        cumulative_net_funding_units: "-20".into(),
        cumulative_other_costs_units: "10".into(),
        held_reserve_units: "20".into(),
        principal_liability_units: "10000000".into(),
        unrealized_pnl_units: "900000000".into(),
        evidence: EVIDENCE_KINDS
            .into_iter()
            .map(|kind| EvidenceRef {
                kind,
                sha256: hash(b"{\"synthetic\":true}"),
            })
            .collect(),
    };
    (
        config,
        Request {
            schema: "esk.game.reconciliation.request.v1".into(),
            statement: sign(statement, SOURCE_DOMAIN, 31),
            previous: None,
        },
    )
}
fn next(config: &Config, prior: &Request) -> Request {
    let candidate = prepare(config, prior, 1000).unwrap();
    let mut source = prior.statement.payload.clone();
    source.sequence = (integer(&source.sequence).unwrap() + 1).to_string();
    source.period_start_ms = source.period_end_ms.clone();
    source.period_end_ms = (integer(&source.period_end_ms).unwrap() + 100).to_string();
    source.previous_source_digest =
        hash(canonical(SOURCE_DOMAIN, &prior.statement.payload).unwrap());
    Request {
        schema: prior.schema.clone(),
        statement: sign(source, SOURCE_DOMAIN, 31),
        previous: Some(Previous {
            statement: prior.statement.clone(),
            settlement: sign(candidate.settlement, SETTLEMENT_DOMAIN, 32),
        }),
    }
}
fn change(request: &mut Request, edit: impl FnOnce(&mut Statement)) {
    edit(&mut request.statement.payload);
    request.statement = sign(request.statement.payload.clone(), SOURCE_DOMAIN, 31);
}

#[test]
fn candidate_matches_actual_main_protocol_and_preserves_unsigned_boundary() {
    let (config, request) = fixture();
    let candidate = prepare(&config, &request, 200).unwrap();
    assert_eq!(candidate.settlement.cumulative_net_profit_units, "2000");
    assert_eq!(
        candidate.calculation.excluded_principal_liability_units,
        "10000000"
    );
    assert!(
        !candidate.settlement_signature_present
            && !candidate.funds_moved
            && !candidate.offchain_payment_authorized
    );
    let payload: elon_game_rewards_harness::model::Settlement =
        serde_json::from_value(serde_json::to_value(&candidate.settlement).unwrap()).unwrap();
    let bytes = elon_game_rewards_harness::policy::canonical("settlement", &payload).unwrap();
    assert_eq!(hex::encode(&bytes), candidate.settlement_signing_bytes_hex);
    let signed = sign(payload, SETTLEMENT_DOMAIN, 32);
    let signed = elon_game_rewards_harness::model::Signed {
        payload: signed.payload,
        signature_hex: signed.signature_hex,
    };
    assert_eq!(
        elon_game_rewards_harness::policy::verify(
            &key(&config.reconciler_public_key_hex).unwrap(),
            "settlement",
            &signed
        )
        .unwrap(),
        hash(bytes)
    );
    assert!(serde_json::from_value::<Signed<settlement::Settlement>>(
        serde_json::to_value(candidate).unwrap()
    )
    .is_err());
}

#[test]
fn loss_recovery_and_reserve_release_remain_cumulative() {
    let (config, mut first) = fixture();
    change(&mut first, |s| {
        s.cumulative_realized_trading_pnl_units = "-100".into()
    });
    assert_eq!(
        prepare(&config, &first, 1000)
            .unwrap()
            .settlement
            .cumulative_net_profit_units,
        "-200"
    );
    let mut second = next(&config, &first);
    change(&mut second, |s| {
        s.cumulative_realized_trading_pnl_units = "300".into()
    });
    assert_eq!(
        prepare(&config, &second, 1000)
            .unwrap()
            .settlement
            .cumulative_net_profit_units,
        "200"
    );
    let mut third = next(&config, &second);
    change(&mut third, |s| s.held_reserve_units = "0".into());
    assert_eq!(
        prepare(&config, &third, 1000)
            .unwrap()
            .settlement
            .cumulative_net_profit_units,
        "220"
    );
}

#[test]
fn principal_and_unrealized_changes_never_create_profit() {
    let (config, mut request) = fixture();
    change(&mut request, |s| {
        s.principal_liability_units = i64::MAX.to_string();
        s.unrealized_pnl_units = i64::MIN.to_string();
    });
    assert_eq!(
        prepare(&config, &request, 1000)
            .unwrap()
            .settlement
            .cumulative_net_profit_units,
        "2000"
    );
}

#[test]
fn every_identity_dimension_is_bound_even_after_valid_resigning() {
    let (config, request) = fixture();
    for field in [
        "source_id",
        "policy_digest",
        "account_scope",
        "user_id",
        "beneficiary",
        "wallet_binding_digest",
        "asset_type",
        "accounting_started_at_ms",
    ] {
        let mut value = serde_json::to_value(&request.statement.payload).unwrap();
        value[field] = "different".into();
        let mut bad = request.clone();
        bad.statement = sign(
            serde_json::from_value::<Statement>(value).unwrap(),
            SOURCE_DOMAIN,
            31,
        );
        assert!(prepare(&config, &bad, 1000).is_err(), "{field}");
    }
    let mut bad = request.clone();
    change(&mut bad, |s| s.asset_decimals = 9);
    assert!(prepare(&config, &bad, 1000).is_err());
}

#[test]
fn unknown_paper_null_missing_and_noncanonical_values_fail() {
    let (config, request) = fixture();
    for (field, value) in [
        ("environment", serde_json::json!("paper")),
        ("basis", serde_json::json!("grid_profit")),
        ("cumulative_trading_fees_units", serde_json::Value::Null),
        ("unknown", serde_json::json!(true)),
    ] {
        let mut json = serde_json::to_value(&request).unwrap();
        json["statement"]["payload"][field] = value;
        assert!(serde_json::from_value::<Request>(json).is_err());
    }
    let mut missing = serde_json::to_value(&request).unwrap();
    missing["statement"]["payload"]
        .as_object_mut()
        .unwrap()
        .remove("cumulative_trading_fees_units");
    assert!(serde_json::from_value::<Request>(missing).is_err());
    for amount in ["01", "1.0", "+1", "-0", "1e3", "", "9223372036854775808"] {
        let mut bad = request.clone();
        change(&mut bad, |s| {
            s.cumulative_realized_trading_pnl_units = amount.into()
        });
        assert!(prepare(&config, &bad, 1000).is_err(), "{amount}");
    }
}

#[test]
fn continuity_requires_both_previous_signatures_and_exact_accounting() {
    let (config, first) = fixture();
    let second = next(&config, &first);
    assert!(prepare(&config, &second, 1000).is_ok());
    let mut no_previous = second.clone();
    no_previous.previous = None;
    assert!(prepare(&config, &no_previous, 1000).is_err());
    for scenario in 0..7 {
        let mut bad = second.clone();
        match scenario {
            0 => change(&mut bad, |s| s.sequence = "3".into()),
            1 => change(&mut bad, |s| s.period_start_ms = "201".into()),
            2 => change(&mut bad, |s| s.previous_source_digest = "99".repeat(32)),
            3 => change(&mut bad, |s| s.cumulative_trading_fees_units = "49".into()),
            4 => change(&mut bad, |s| s.cumulative_other_costs_units = "9".into()),
            5 => bad.previous.as_mut().unwrap().settlement.signature_hex = "00".repeat(64),
            _ => {
                let prior = bad.previous.as_mut().unwrap();
                prior.settlement.payload.cumulative_net_profit_units = "9999".into();
                prior.settlement = sign(prior.settlement.payload.clone(), SETTLEMENT_DOMAIN, 32);
            }
        }
        assert!(prepare(&config, &bad, 1000).is_err(), "scenario {scenario}");
    }
}

#[test]
fn invalid_signatures_and_incomplete_evidence_fail_closed() {
    let (config, request) = fixture();
    for scenario in 0..7 {
        let mut bad = request.clone();
        match scenario {
            0 => bad.statement.signature_hex = "00".repeat(64),
            1 => bad.statement = sign(bad.statement.payload, SOURCE_DOMAIN, 32),
            2 => change(&mut bad, |s| {
                s.evidence.pop();
            }),
            3 => change(&mut bad, |s| s.evidence.swap(0, 1)),
            4 => change(&mut bad, |s| s.evidence[0].sha256 = "0".repeat(64)),
            5 => change(&mut bad, |s| s.period_end_ms = "1001".into()),
            _ => change(&mut bad, |s| s.period_start_ms = "101".into()),
        }
        assert!(prepare(&config, &bad, 1000).is_err(), "scenario {scenario}");
    }
}

#[test]
fn arithmetic_rejects_final_overflow_but_accepts_wide_cancellation() {
    let (config, mut request) = fixture();
    change(&mut request, |s| {
        s.cumulative_realized_trading_pnl_units = i64::MAX.to_string();
        s.cumulative_net_funding_units = i64::MAX.to_string();
    });
    assert!(prepare(&config, &request, 1000).is_err());
    change(&mut request, |s| {
        s.cumulative_trading_fees_units = i64::MAX.to_string()
    });
    assert_eq!(
        prepare(&config, &request, 1000)
            .unwrap()
            .settlement
            .cumulative_net_profit_units,
        (i64::MAX - 30).to_string()
    );
    change(&mut request, |s| {
        s.cumulative_realized_trading_pnl_units = i64::MIN.to_string();
        s.cumulative_net_funding_units = "-1".into();
    });
    assert!(prepare(&config, &request, 1000).is_err());
}
