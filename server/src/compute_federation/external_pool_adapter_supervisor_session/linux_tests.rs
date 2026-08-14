use std::{mem::MaybeUninit, os::fd::RawFd, thread};

use anyhow::Result;

use super::{
    prepare_external_pool_adapter_supervisor_session, AuthenticatedExternalPoolAdapterSession,
    ExternalPoolAdapterSessionFrameKind, ExternalPoolAdapterSessionRoots,
};

#[test]
fn linux_kernel_uses_anonymous_cloexec_seqpacket_and_one_time_seed_pipe() {
    let prepared = prepare_external_pool_adapter_supervisor_session(roots(0x11))
        .expect("prepare V260 session");
    let (host_bootstrap, child_bootstrap) = prepared.split();

    assert_eq!(
        socket_option(child_bootstrap.socket_fd_for_test(), libc::SO_TYPE),
        libc::SOCK_SEQPACKET
    );
    assert_eq!(
        socket_option(child_bootstrap.socket_fd_for_test(), libc::SO_ACCEPTCONN),
        0
    );
    assert_cloexec(child_bootstrap.socket_fd_for_test());
    assert_cloexec(child_bootstrap.seed_fd_for_test());
    assert_pipe(child_bootstrap.seed_fd_for_test());

    let child_thread = thread::spawn(move || child_bootstrap.authenticate(roots(0x11)));
    let mut host = host_bootstrap.authenticate().expect("authenticate host");
    let mut child = child_thread
        .join()
        .expect("join child bootstrap")
        .expect("authenticate child");

    host.send(
        ExternalPoolAdapterSessionFrameKind::Control,
        br#"{"op":"ping"}"#,
    )
    .expect("send host control frame");
    let frame = child.receive().expect("receive child control frame");
    assert!(frame.kind() == ExternalPoolAdapterSessionFrameKind::Control);
    assert_eq!(frame.payload(), br#"{"op":"ping"}"#);

    child
        .send(
            ExternalPoolAdapterSessionFrameKind::Config,
            &[0x00, 0xff, 0x7f],
        )
        .expect("send child config frame");
    let frame = host.receive().expect("receive host config frame");
    assert!(frame.kind() == ExternalPoolAdapterSessionFrameKind::Config);
    assert_eq!(frame.payload(), &[0x00, 0xff, 0x7f]);
}

#[test]
fn bootstrap_rejects_root_mismatch_wrong_seed_and_tampered_proof() {
    assert_bootstrap_rejected(|child| child.authenticate(roots(0x22)), 0x11);
    assert_bootstrap_rejected(
        |child| child.authenticate_with_wrong_seed_for_test(roots(0x11)),
        0x11,
    );
    assert_bootstrap_rejected(
        |child| child.authenticate_with_tampered_proof_for_test(roots(0x11)),
        0x11,
    );
}

#[test]
fn authenticated_frames_reject_tamper_and_become_terminal() {
    let (mut host, child) = authenticated_pair();
    let mut packet = child.encode_outgoing_for_test(1, 1, b"tamper-me");
    let last = packet.len() - 1;
    packet[last] ^= 0x80;
    child
        .send_raw_for_test(&packet)
        .expect("send tampered packet");

    assert!(host.receive().is_err());
    assert!(host
        .send(
            ExternalPoolAdapterSessionFrameKind::Control,
            b"after-failure"
        )
        .is_err());
    assert!(host.receive().is_err());
}

#[test]
fn authenticated_frames_reject_replay_and_out_of_order_sequences() {
    let (mut host, child) = authenticated_pair();
    let packet = child.encode_outgoing_for_test(1, 1, b"once");
    child.send_raw_for_test(&packet).expect("send first packet");
    child.send_raw_for_test(&packet).expect("replay packet");
    assert_eq!(
        host.receive().expect("receive first packet").payload(),
        b"once"
    );
    assert!(host.receive().is_err());

    let (mut host, child) = authenticated_pair();
    let packet = child.encode_outgoing_for_test(1, 2, b"skip-sequence-one");
    child
        .send_raw_for_test(&packet)
        .expect("send out-of-order packet");
    assert!(host.receive().is_err());
}

#[test]
fn authenticated_frames_reject_reflection_unknown_kind_and_credential_oversize() {
    let (mut host, child) = authenticated_pair();
    let reflected = child.encode_incoming_for_test(1, 1, b"reflected");
    child
        .send_raw_for_test(&reflected)
        .expect("send reflected packet");
    assert!(host.receive().is_err());

    let (mut host, child) = authenticated_pair();
    let unknown = child.encode_outgoing_for_test(99, 1, b"unknown-kind");
    child
        .send_raw_for_test(&unknown)
        .expect("send unknown kind");
    assert!(host.receive().is_err());

    let (mut host, child) = authenticated_pair();
    let oversized = vec![0x5a; 65_537];
    let packet = child.encode_outgoing_for_test(3, 1, &oversized);
    child
        .send_raw_for_test(&packet)
        .expect("send oversized credential frame");
    assert!(host.receive().is_err());
}

fn authenticated_pair() -> (
    AuthenticatedExternalPoolAdapterSession,
    AuthenticatedExternalPoolAdapterSession,
) {
    let prepared = prepare_external_pool_adapter_supervisor_session(roots(0x11))
        .expect("prepare authenticated pair");
    let (host_bootstrap, child_bootstrap) = prepared.split();
    let child_thread = thread::spawn(move || child_bootstrap.authenticate(roots(0x11)));
    let host = host_bootstrap.authenticate().expect("authenticate host");
    let child = child_thread
        .join()
        .expect("join child bootstrap")
        .expect("authenticate child");
    (host, child)
}

fn assert_bootstrap_rejected<F>(child_authenticate: F, host_root: u8)
where
    F: FnOnce(
            super::ExternalPoolAdapterChildBootstrap,
        ) -> Result<AuthenticatedExternalPoolAdapterSession>
        + Send
        + 'static,
{
    let prepared = prepare_external_pool_adapter_supervisor_session(roots(host_root))
        .expect("prepare rejected bootstrap");
    let (host_bootstrap, child_bootstrap) = prepared.split();
    let child_thread = thread::spawn(move || child_authenticate(child_bootstrap));
    assert!(host_bootstrap.authenticate().is_err());
    assert!(child_thread
        .join()
        .expect("join rejected bootstrap")
        .is_err());
}

fn roots(marker: u8) -> ExternalPoolAdapterSessionRoots {
    let policy = digest(0x77);
    let profile = digest(marker);
    let target = digest(0x33);
    let companion = digest(0x44);
    let capsule = digest(0x55);
    let bundle = digest(0x66);
    ExternalPoolAdapterSessionRoots::new(&policy, &profile, &target, &companion, &capsule, &bundle)
        .expect("construct exact V260 roots")
}

fn digest(byte: u8) -> String {
    format!("{byte:02x}").repeat(32)
}

fn socket_option(fd: RawFd, option: libc::c_int) -> libc::c_int {
    let mut value = 0;
    let mut length = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
    assert_eq!(
        unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                option,
                (&mut value as *mut libc::c_int).cast(),
                &mut length,
            )
        },
        0
    );
    assert_eq!(length as usize, std::mem::size_of::<libc::c_int>());
    value
}

fn assert_cloexec(fd: RawFd) {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    assert_ne!(flags, -1);
    assert_ne!(flags & libc::FD_CLOEXEC, 0);
}

fn assert_pipe(fd: RawFd) {
    let mut stat = MaybeUninit::<libc::stat>::zeroed();
    assert_eq!(unsafe { libc::fstat(fd, stat.as_mut_ptr()) }, 0);
    let stat = unsafe { stat.assume_init() };
    assert_eq!(stat.st_mode & libc::S_IFMT, libc::S_IFIFO);
}
