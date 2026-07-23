use super::*;

fn process(pid: u32, parent_pid: u32, name: &str, path: Option<&str>) -> ProcessIdentity {
    ProcessIdentity {
        pid,
        parent_pid,
        name: name.to_string(),
        image_path: path.map(ToOwned::to_owned),
    }
}

#[test]
fn trusted_desktop_chain_is_accepted_but_same_sid_executor_chain_is_denied() {
    let desktop = vec![
        process(10, 20, "powershell.exe", None),
        process(20, 30, "codex-code-mode-host.exe", None),
        process(30, 40, "codex.exe", None),
        process(
            40,
            1,
            "ChatGPT.exe",
            Some(
                r"C:\Program Files\WindowsApps\OpenAI.Codex_26.715.1.0_x64__2p2nqsd0c76g0\app\ChatGPT.exe",
            ),
        ),
    ];
    assert_eq!(trusted_desktop_ancestry(10, &desktop), Ok(()));

    let executor = vec![
        process(10, 20, "powershell.exe", None),
        process(20, 30, "codex.exe", None),
        process(30, 40, "elon-cli-worker.exe", None),
        process(40, 1, "一龙开发平台.exe", None),
    ];
    assert_eq!(
        trusted_desktop_ancestry(10, &executor),
        Err("desktop_review_executor_ancestry_denied")
    );
}

#[test]
fn lookalike_or_missing_desktop_package_fails_closed() {
    for path in [
        r"C:\Temp\OpenAI.Codex_fake\app\ChatGPT.exe",
        r"C:\Program Files\WindowsApps\Other.App_1.0\app\ChatGPT.exe",
    ] {
        let chain = vec![process(10, 0, "ChatGPT.exe", Some(path))];
        assert_eq!(
            trusted_desktop_ancestry(10, &chain),
            Err("desktop_review_caller_not_codex_desktop")
        );
    }
}

#[test]
fn broker_signature_is_v3_and_bound_to_canonical_review_path() {
    let broker = DesktopReviewBroker::generate("install-test").unwrap();
    let inner = broker.inner.unwrap();
    let body = br#"{"verdict":"accepted","summary":"independently verified"}"#;
    let request = SignRequest {
        protocol: PROTOCOL.to_string(),
        owner_user_id: "owner".to_string(),
        task_id: "local-test".to_string(),
        method: "POST".to_string(),
        endpoint_path: "/api/local-tasks/local-test/supervision/desktop-review".to_string(),
        body_sha256: hex::encode(Sha256::digest(body)),
    };
    let ticket = inner.sign(&request).unwrap();
    assert!(ticket.starts_with(&format!("v3.{}.", inner.key_id)));
    let ledger = std::env::temp_dir().join(format!(
        "elon-review-broker-ledger-{}.json",
        uuid::Uuid::new_v4()
    ));
    let auth = crate::node_agent_desktop_review_auth::DesktopReviewAuth::for_v3_test_key(
        &inner.key_id,
        inner.private_key.to_public_key(),
        ledger.clone(),
    );
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        crate::node_agent_desktop_review_auth::DESKTOP_REVIEW_TICKET_HEADER,
        ticket.parse().unwrap(),
    );
    assert_eq!(
        auth.verify_and_consume(
            &headers,
            "owner",
            "local-test",
            "POST",
            "/api/local-tasks/local-test/supervision/desktop-review",
            b"{}",
        ),
        Err(crate::node_agent_desktop_review_auth::DesktopReviewAuthError::Invalid)
    );
    assert_eq!(
        auth.verify_and_consume(
            &headers,
            "owner",
            "local-test",
            "POST",
            "/api/local-tasks/local-test/supervision/desktop-review",
            body,
        ),
        Ok(())
    );
    let _ = std::fs::remove_file(ledger);
    let mut changed = request;
    changed.endpoint_path = "/other".to_string();
    assert_eq!(
        inner.sign(&changed),
        Err("desktop_review_broker_request_invalid")
    );
}
