const SESSION_ROOTS: &str =
    include_str!("../../../external-pool-adapter-session-core/src/roots/task_production.rs");
const ROOTS_PARENT: &str = include_str!("../../../external-pool-adapter-session-core/src/roots.rs");
const ROOT_ARGUMENTS: &str =
    include_str!("../../../external-pool-adapter-session-core/src/roots/root_arguments.rs");
const SESSION_WIRE: &str =
    include_str!("../../../external-pool-adapter-session-core/src/task_protocol/wire.rs");
const SESSION_TRANSPORT: &str =
    include_str!("../../../external-pool-adapter-session-core/src/transport.rs");
const SESSION_HOST: &str =
    include_str!("../../../external-pool-adapter-session-core/src/task_protocol/host.rs");
const DOMAIN_SESSION: &str =
    include_str!("../external_pool_adapter_task_protocol_production/session.rs");
const SESSION_FACADE: &str = include_str!("../external_pool_adapter_supervisor_session.rs");
const LAUNCH_ROOT: &str = include_str!("../external_pool_adapter_linux_supervisor/launch.rs");
const LAUNCH_ARGUMENTS: &str = include_str!(
    "../external_pool_adapter_linux_supervisor/launch/task_protocol_production_arguments.rs"
);
const BROKER_ROOT: &str = include_str!("../external_pool_adapter_broker_tls.rs");
const BROKER_TASK: &str = include_str!("../external_pool_adapter_broker_tls/task_protocol.rs");
const TASK_DELIVERY: &str =
    include_str!("../../store/compute_external_pool_adapter_runtime_bundle/task_delivery.rs");

const ROOTS: &[&str] = &[
    "supervisor_session_policy_digest",
    "runtime_launch_profile_digest",
    "task_protocol_profile_digest",
    "upstream_transport_target_digest",
    "supervisor_session_policy_companion_digest",
    "launch_image_sha256",
    "ephemeral_task_secret_delivery_root",
    "task_protocol_conformance_run_receipt_digest",
];

const ARGUMENTS: &[&str] = &[
    "--elon-task-production-policy=",
    "--elon-task-production-runtime-profile=",
    "--elon-task-production-protocol-profile=",
    "--elon-task-production-target=",
    "--elon-task-production-companion=",
    "--elon-task-production-launch-image=",
    "--elon-task-production-secret-delivery=",
    "--elon-task-production-conformance-receipt=",
];

#[test]
fn task_protocol_production_session_freezes_eight_roots_domains_and_arguments() {
    assert_ordered(SESSION_ROOTS, ROOTS);
    assert_ordered(DOMAIN_SESSION, ROOTS);
    for source in [SESSION_ROOTS, DOMAIN_SESSION] {
        assert!(source
            .contains("elon.external_pool_adapter.task_protocol.production.session.roots.v1\\0"));
        assert!(source.contains(
            "elon.external_pool_adapter.task_protocol.production.session.kdf_salt.v1\\0"
        ));
    }
    assert!(ROOTS_PARENT.contains("TaskProtocolProduction("));
    assert!(ROOTS_PARENT.contains("pub fn new_task_protocol_production("));
    assert!(ROOT_ARGUMENTS.contains("TaskProtocolProduction([String; 8])"));
    assert!(ROOT_ARGUMENTS.contains("task_protocol_production_values"));
    assert_ordered(LAUNCH_ARGUMENTS, ARGUMENTS);
    assert_ordered(DOMAIN_SESSION, ARGUMENTS);
    assert_eq!(
        LAUNCH_ARGUMENTS.matches("--elon-task-production-").count(),
        8
    );
    assert!(LAUNCH_ARGUMENTS.contains("ROOT_ARGUMENT_PREFIXES: [&str; 8]"));
    assert!(LAUNCH_ROOT.contains("roots.task_protocol_production_values()"));
    assert!(
        SESSION_FACADE.contains("external_pool_adapter_task_protocol_production_session_roots(")
    );
    assert_ordered(
        SESSION_FACADE,
        &[
            "server_supervisor_session_policy_catalog()?",
            "ExternalPoolAdapterSessionRoots::new_task_protocol_production(",
        ],
    );
}

#[test]
fn task_protocol_production_reuses_exact_eltp_v1_and_bounded_receipt_only_tls() {
    for marker in [
        "const MAGIC: &[u8; 4] = b\"ELTP\"",
        "const VERSION: u8 = 1",
        "const FLAGS: u16 = 0",
        "MAX_UPSTREAM_REQUEST_BYTES: usize = 65_536",
        "MAX_UPSTREAM_RESPONSE_BYTES: usize = 262_144",
        "MAX_EXCHANGE_ORDINAL: u64 = 64",
        "elon.external_pool_adapter.task_protocol.request.v1\\0",
        "elon.external_pool_adapter.task_protocol.exchange.v1\\0",
    ] {
        assert!(SESSION_WIRE.contains(marker), "ELTP v1 lost {marker}");
    }
    assert!(BROKER_ROOT.contains("#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]"));
    assert!(BROKER_ROOT.contains("mod task_protocol;"));
    assert!(BROKER_ROOT.contains("exchange_external_pool_adapter_broker_task"));
    assert_ordered(
        BROKER_TASK,
        &[
            "const MAX_REQUEST_BYTES: usize = 65_536",
            "const MAX_RESPONSE_BYTES: usize = 262_144",
            "let stream = channel.begin_application_exchange()?",
            "Zeroizing::new",
            "let timeout = exchange.remaining_timeout()?",
            "stream.write_all(request).await?",
            "stream.flush().await?",
            "stream.read_exact(&mut response[..]).await?",
            "exchange.complete(&response, validate_observation)?",
            "Ok(receipt)",
        ],
    );
    assert_eq!(
        BROKER_TASK.matches("begin_application_exchange()?").count(),
        1
    );
    assert!(BROKER_TASK.contains("FnOnce(&[u8]) -> Result<()> + Send"));
    assert!(BROKER_TASK.contains("Result<ExternalPoolAdapterTaskProtocolHostReceipt>"));
    assert!(BROKER_TASK.contains("let timeout = exchange.remaining_timeout()?"));
    assert!(!BROKER_TASK.contains("timeout: Duration"));
    assert!(!BROKER_TASK.contains("MAX_EXCHANGE_TIMEOUT"));
    for forbidden in ["TcpStream", "TlsStream", "Vec<u8>>", "Result<&[u8]>"] {
        assert!(
            !BROKER_TASK.contains(forbidden),
            "relay exposed {forbidden}"
        );
    }
}

#[test]
fn task_protocol_production_delivery_keeps_one_host_per_session_and_no_v213_constructor() {
    assert!(TASK_DELIVERY.contains("host: &mut ExternalPoolAdapterTaskProtocolHost<'_>"));
    assert_ordered(
        TASK_DELIVERY,
        &[
            "const MAX_TOTAL_EXCHANGE_TIMEOUT: Duration = Duration::from_millis(15_000)",
            "timeout.is_zero() || timeout > MAX_TOTAL_EXCHANGE_TIMEOUT",
            "let exchange = host.begin(request, delivery_attempt_digest, timeout)?",
            "exchange_external_pool_adapter_broker_task(",
        ],
    );
    assert!(!TASK_DELIVERY.contains("checked_div"));
    assert!(!TASK_DELIVERY.contains("ExternalPoolAdapterTaskProtocolHost::new"));
    for forbidden in [
        "PreparedStartSendRequest",
        "CommittedStartSendAuthority",
        "VerifiedComputeStartOutboxRemoteObservation",
        "AcceptedComputeStartOutbox",
    ] {
        let combined = format!("{BROKER_TASK}{TASK_DELIVERY}");
        assert!(
            !combined.contains(forbidden),
            "dormant relay gained {forbidden}"
        );
    }
    for forbidden in [
        ".unwrap(",
        ".expect(",
        "panic!",
        "todo!",
        "unimplemented!",
        "unreachable!",
    ] {
        let production = format!(
            "{SESSION_ROOTS}{DOMAIN_SESSION}{LAUNCH_ARGUMENTS}{BROKER_TASK}{TASK_DELIVERY}"
        );
        assert!(
            !production.contains(forbidden),
            "production retained {forbidden}"
        );
    }
}

#[test]
fn task_protocol_production_exchange_shares_one_absolute_deadline() {
    assert!(SESSION_TRANSPORT
        .contains("const MAX_FRAME_IO_TIMEOUT: Duration = Duration::from_millis(15_000)"));
    assert_ordered(
        SESSION_TRANSPORT,
        &[
            "pub(crate) fn send_with_timeout(",
            "timeout.is_zero() || timeout > MAX_FRAME_IO_TIMEOUT",
            "self.send_inner(kind, payload, timeout)",
            "send_packet(self.socket.as_raw_fd(), &packet, timeout)?",
        ],
    );
    assert!(SESSION_TRANSPORT.contains("self.send_with_timeout(kind, payload, FRAME_IO_TIMEOUT)"));
    assert_ordered(
        SESSION_HOST,
        &[
            "const MAX_EXCHANGE_TIMEOUT: Duration = Duration::from_millis(15_000)",
            "Instant::now().checked_add(timeout)",
            "remaining_before(deadline)",
            "self.session.send_with_timeout(",
            "self.session.receive_with_timeout(receive_timeout)?",
            "deadline,",
            "pub fn remaining_timeout(&self) -> Result<Duration>",
            "remaining_before(self.deadline)",
            "self.session.send_with_timeout(",
            "self.session.receive_with_timeout(receive_timeout)?",
        ],
    );
    assert!(SESSION_HOST.contains("checked_duration_since(Instant::now())"));
    assert!(SESSION_HOST.contains("timeout <= MAX_EXCHANGE_TIMEOUT"));
    assert_ordered(
        SESSION_HOST,
        &[
            "require_before_deadline(self.session, self.deadline)?",
            "let observation = match validate_observation(&received.observation)",
            "require_before_deadline(self.session, self.deadline)?",
            "let receipt = ExternalPoolAdapterTaskProtocolHostReceipt",
            "require_before_deadline(self.session, self.deadline)?",
            "self.active = false",
        ],
    );
    assert_eq!(
        SESSION_HOST
            .matches("require_before_deadline(self.session, self.deadline)?")
            .count(),
        3
    );
    assert!(SESSION_HOST.contains("must be pure and bounded"));
    assert!(SESSION_HOST.contains("executes synchronously and is not"));
    assert!(SESSION_HOST.contains("preempted; deadline checks around it"));
    assert!(TASK_DELIVERY.contains("semantic validator must be pure and bounded"));
    assert!(TASK_DELIVERY.contains("synchronous validation is not preempted"));
}

fn assert_ordered(source: &str, needles: &[&str]) {
    let mut cursor = 0;
    for needle in needles {
        let offset = source[cursor..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing ordered source marker {needle}"));
        cursor += offset + needle.len();
    }
}
