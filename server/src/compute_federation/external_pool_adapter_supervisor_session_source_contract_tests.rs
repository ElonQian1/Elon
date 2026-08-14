const COMPUTE_FEDERATION_MOD: &str = include_str!("mod.rs");
const SESSION_FACADE: &str = include_str!("external_pool_adapter_supervisor_session.rs");
const SESSION_ROOTS: &str = include_str!("external_pool_adapter_supervisor_session/roots.rs");
const SESSION_CRYPTO: &str = include_str!("external_pool_adapter_supervisor_session/crypto.rs");
const SESSION_BOOTSTRAP: &str =
    include_str!("external_pool_adapter_supervisor_session/bootstrap.rs");
const SESSION_TRANSPORT: &str =
    include_str!("external_pool_adapter_supervisor_session/transport.rs");

#[test]
fn v260_is_linux_x86_64_only_and_has_no_runtime_route() {
    assert!(COMPUTE_FEDERATION_MOD.contains(
        "#[cfg(all(target_os = \"linux\", target_arch = \"x86_64\"))]\npub(crate) mod external_pool_adapter_supervisor_session;"
    ));
    assert!(COMPUTE_FEDERATION_MOD
        .contains("mod external_pool_adapter_supervisor_session_source_contract_tests;"));
    assert!(SESSION_FACADE.contains("It does not launch a"));
    assert!(!include_str!("../main.rs").contains("external_pool_adapter_supervisor_session"));
    assert!(!include_str!("../router.rs").contains("external_pool_adapter_supervisor_session"));
}

#[test]
fn v260_binds_all_durable_roots_to_v259_policy() {
    for required in [
        "server_supervisor_session_policy_catalog()",
        "anonymous_child_socketpair_seqpacket_v1",
        "elon.external_pool_adapter.sidecar.v1",
        "hkdf_sha256_extract_expand_v1",
        "hmac_sha256_32_v1",
        "policy_digest: [u8; 32]",
        "profile_digest: [u8; 32]",
        "target_digest: [u8; 32]",
        "companion_digest: [u8; 32]",
        "capsule_digest: [u8; 32]",
        "bundle_digest: [u8; 32]",
        "b\"policy\\0\"",
        "b\"profile\\0\"",
        "b\"target\\0\"",
        "b\"companion\\0\"",
        "b\"capsule\\0\"",
        "b\"bundle\\0\"",
        "exact lowercase SHA-256 hex",
        "digest must not be all zero",
    ] {
        assert!(
            SESSION_ROOTS.contains(required),
            "missing bound root {required}"
        );
    }
}

#[test]
fn v260_uses_os_random_hkdf_hmac_and_non_copying_secrets() {
    for required in [
        "ring::rand::SystemRandom::new()",
        "hkdf::HKDF_SHA256",
        "HmacSha256",
        "HOST_TO_CHILD_DIRECTION",
        "CHILD_TO_HOST_DIRECTION",
        "struct Secret32(Zeroizing<[u8; 32]>)",
        "fn zeroize_now",
        "mac.verify_slice(expected_tag)",
    ] {
        assert!(
            SESSION_CRYPTO.contains(required),
            "missing crypto rule {required}"
        );
    }
    let secret_tail = SESSION_CRYPTO
        .split_once("struct Secret32")
        .expect("Secret32 remains defined")
        .0;
    assert!(!secret_tail.ends_with("#[derive(Clone)]\n"));
    assert!(!secret_tail.ends_with("#[derive(Copy, Clone)]\n"));
    assert!(!SESSION_CRYPTO.contains("Serialize"));
    assert!(!SESSION_CRYPTO.contains("Deserialize"));
}

#[test]
fn v260_bootstrap_is_anonymous_fixed_size_and_mutually_authenticated() {
    for required in [
        "libc::pipe2(descriptors.as_mut_ptr(), libc::O_CLOEXEC)",
        "create_seqpacket_pair()?",
        "const CHALLENGE_BYTES: usize = 4 + 1 + 32 + 32",
        "const RESPONSE_BYTES: usize = RESPONSE_PREFIX_BYTES + 32",
        "const CONFIRM_BYTES: usize = CONFIRM_PREFIX_BYTES + 32",
        "read_seed(self.seed_reader)?",
        "seed.len()",
        "trailing_read != 0",
        "bootstrap_response\\0",
        "bootstrap_confirm\\0",
        "constant_time::verify_slices_are_equal",
        "libc::shutdown(socket.as_raw_fd(), libc::SHUT_RDWR)",
    ] {
        assert!(
            SESSION_BOOTSTRAP.contains(required),
            "missing bootstrap rule {required}"
        );
    }
    for forbidden in [
        "std::env",
        "File::open",
        "OpenOptions",
        "TcpStream",
        "TcpListener",
    ] {
        assert!(
            !SESSION_BOOTSTRAP.contains(forbidden),
            "bootstrap crosses boundary {forbidden}"
        );
    }
}

#[test]
fn v260_elsp_frames_are_bounded_ordered_directional_and_fail_closed() {
    for required in [
        "const FRAME_MAGIC: &[u8; 4] = b\"ELSP\"",
        "const FRAME_HEADER_BYTES: usize = 20",
        "const FRAME_TAG_BYTES: usize = 32",
        "const MAX_CONTROL_PAYLOAD_BYTES: usize = 1_048_576",
        "const MAX_CONFIG_PAYLOAD_BYTES: usize = 1_048_576",
        "const MAX_CREDENTIAL_PAYLOAD_BYTES: usize = 65_536",
        "const MAX_FRAMES_PER_DIRECTION: u64 = 1_048_576",
        "SOCK_SEQPACKET | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK",
        "libc::socketpair(libc::AF_UNIX",
        "libc::MSG_TRUNC",
        "vec![0_u8; maximum_bytes]",
        "received > maximum_bytes",
        "sequence != self.next_receive_sequence",
        "verify_mac(",
        "self.send_key.zeroize_now()",
        "self.receive_key.zeroize_now()",
        "self.active = false",
        "authenticated session is terminal",
    ] {
        assert!(
            SESSION_TRANSPORT.contains(required),
            "missing frame rule {required}"
        );
    }
    assert!(
        SESSION_TRANSPORT
            .find("verify_mac(")
            .expect("MAC verification")
            < SESSION_TRANSPORT
                .find("from_byte(header[5])")
                .expect("kind parsing")
    );
}

#[test]
fn v260_has_no_process_network_persistence_activation_or_real_secret_effect() {
    let production = [
        SESSION_FACADE,
        SESSION_ROOTS,
        SESSION_CRYPTO,
        SESSION_BOOTSTRAP,
        SESSION_TRANSPORT,
    ]
    .concat();
    for forbidden in [
        "std::process",
        "tokio::process",
        "Command::",
        "execve",
        "posix_spawn",
        "fork(",
        "TcpStream",
        "TcpListener",
        "reqwest::",
        "INSERT INTO",
        "UPDATE compute_",
        "DELETE FROM",
        "with_sensitive_bytes",
        "activate_external_pool",
        "settlement",
        "blockchain",
    ] {
        assert!(
            !production.contains(forbidden),
            "V260 crosses no-effect fence {forbidden}"
        );
    }
}
