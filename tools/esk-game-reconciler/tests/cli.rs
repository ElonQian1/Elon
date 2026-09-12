use ed25519_dalek::{Signer, SigningKey};
use esk_game_reconciler::{files, model::*, settlement::Signed, verify::*};
use std::{fs, path::Path, process::Command};

fn setup(root: &Path) {
    let address = format!("0x{}", "22".repeat(32));
    let source_key = SigningKey::from_bytes(&[31; 32]);
    let c = Config {
        schema: "esk.game.reconciliation.config.v1".into(),
        policy_digest: "11".repeat(32),
        source_id: "fixture".into(),
        source_public_key_hex: hex::encode(source_key.verifying_key().as_bytes()),
        reconciler_public_key_hex: hex::encode(
            SigningKey::from_bytes(&[32; 32]).verifying_key().as_bytes(),
        ),
        account_scope: "account-1".into(),
        user_id: "user-1".into(),
        beneficiary: address.clone(),
        wallet_binding_digest: "33".repeat(32),
        asset_type: format!("{address}::usd::USD"),
        asset_decimals: 6,
        accounting_started_at_ms: "100".into(),
    };
    let blob = br#"{"synthetic":true}"#;
    let blob_hash = hash(blob);
    let s = Statement {
        schema: "esk.game.reconciliation.source.v1".into(),
        source_id: c.source_id.clone(),
        environment: Environment::Live,
        basis: Basis::ParticipantCumulativeCashV1,
        policy_digest: c.policy_digest.clone(),
        account_scope: c.account_scope.clone(),
        user_id: c.user_id.clone(),
        beneficiary: c.beneficiary.clone(),
        wallet_binding_digest: c.wallet_binding_digest.clone(),
        asset_type: c.asset_type.clone(),
        asset_decimals: 6,
        accounting_started_at_ms: "100".into(),
        sequence: "1".into(),
        previous_source_digest: "0".repeat(64),
        period_start_ms: "100".into(),
        period_end_ms: "200".into(),
        cumulative_realized_trading_pnl_units: "100".into(),
        cumulative_trading_fees_units: "3".into(),
        cumulative_net_funding_units: "-2".into(),
        cumulative_other_costs_units: "1".into(),
        held_reserve_units: "4".into(),
        principal_liability_units: "10000000".into(),
        unrealized_pnl_units: "50000000".into(),
        evidence: EVIDENCE_KINDS
            .into_iter()
            .map(|kind| EvidenceRef {
                kind,
                sha256: blob_hash.clone(),
            })
            .collect(),
    };
    let signature_hex = hex::encode(
        source_key
            .sign(&canonical(SOURCE_DOMAIN, &s).unwrap())
            .to_bytes(),
    );
    let r = Request {
        schema: "esk.game.reconciliation.request.v1".into(),
        statement: Signed {
            payload: s,
            signature_hex,
        },
        previous: None,
    };
    fs::write(root.join("config.json"), serde_json::to_vec(&c).unwrap()).unwrap();
    fs::write(root.join("request.json"), serde_json::to_vec(&r).unwrap()).unwrap();
    fs::create_dir(root.join("evidence")).unwrap();
    fs::write(
        root.join("evidence").join(format!("{blob_hash}.json")),
        blob,
    )
    .unwrap();
}
fn command(root: &Path) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_esk-game-reconciler"));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd.arg(root.join("config.json"))
        .arg(root.join("request.json"))
        .arg(root.join("evidence"))
        .arg(root.join("candidate.json"))
        .output()
        .unwrap()
}
fn evidence_path(root: &Path) -> std::path::PathBuf {
    fs::read_dir(root.join("evidence"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
}

#[test]
fn real_cli_writes_one_unsigned_candidate_and_preserves_existing_output() {
    let tmp = tempfile::tempdir().unwrap();
    setup(tmp.path());
    let result = command(tmp.path());
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let bytes = fs::read(tmp.path().join("candidate.json")).unwrap();
    let output: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(output["settlement"]["cumulative_net_profit_units"], "90");
    assert_eq!(output["funds_moved"], false);
    assert_eq!(output["settlement_signature_present"], false);
    assert!(!command(tmp.path()).status.success());
    assert_eq!(fs::read(tmp.path().join("candidate.json")).unwrap(), bytes);
}

#[test]
fn absent_tampered_and_oversized_archives_do_not_create_output() {
    for mode in 0..3 {
        let tmp = tempfile::tempdir().unwrap();
        setup(tmp.path());
        let evidence = evidence_path(tmp.path());
        match mode {
            0 => {
                fs::rename(&evidence, evidence.with_extension("unavailable")).unwrap();
            }
            1 => fs::write(&evidence, b"changed").unwrap(),
            _ => fs::write(&evidence, vec![0; files::MAX_EVIDENCE_BYTES + 1]).unwrap(),
        }
        assert!(!command(tmp.path()).status.success());
        assert!(!tmp.path().join("candidate.json").exists());
    }
}

#[test]
fn malformed_inputs_do_not_echo_contents_or_create_output() {
    for content in [
        b"SENSITIVE_SENTINEL".to_vec(),
        vec![b' '; files::MAX_REQUEST_BYTES + 1],
    ] {
        let tmp = tempfile::tempdir().unwrap();
        setup(tmp.path());
        fs::write(tmp.path().join("request.json"), content).unwrap();
        let result = command(tmp.path());
        assert!(!result.status.success());
        assert!(result.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("SENSITIVE_SENTINEL"));
        assert!(!tmp.path().join("candidate.json").exists());
    }
}

#[test]
fn output_input_collision_preserves_input_and_concurrent_writers_do_not_replace() {
    let tmp = tempfile::tempdir().unwrap();
    setup(tmp.path());
    let config = tmp.path().join("config.json");
    let request = tmp.path().join("request.json");
    let archive = tmp.path().join("evidence");
    let before = fs::read(&request).unwrap();
    assert!(files::run(&config, &request, &archive, &request, 200).is_err());
    assert_eq!(fs::read(&request).unwrap(), before);
    let result = std::thread::scope(|scope| {
        let a = scope.spawn(|| command(tmp.path()));
        let b = scope.spawn(|| command(tmp.path()));
        [
            a.join().unwrap().status.success(),
            b.join().unwrap().status.success(),
        ]
    });
    assert_eq!(result.into_iter().filter(|success| *success).count(), 1);
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(tmp.path().join("candidate.json")).unwrap()).unwrap();
    assert_eq!(value["schema"], "esk.game.reconciliation.candidate.v1");
}
