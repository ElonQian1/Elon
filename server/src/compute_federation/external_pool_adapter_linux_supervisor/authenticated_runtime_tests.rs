use std::{
    collections::BTreeSet,
    fs::{self, File},
    path::{Path, PathBuf},
    time::Duration,
};

use super::{
    launch_external_pool_adapter_supervisor_child, ExternalPoolAdapterSupervisorCgroupParent,
    ExternalPoolAdapterSupervisorChild,
};
use crate::compute_federation::external_pool_adapter_supervisor_session::{
    external_pool_adapter_session_roots, prepare_external_pool_adapter_supervisor_session,
    ExternalPoolAdapterChildBootstrap, ExternalPoolAdapterSessionFrameKind,
};
use crate::store::compute_external_pool_adapter_runtime_bundle::with_materialized_external_pool_adapter_test_capsule;
use elon_external_pool_adapter_session_core::{
    prepare_external_pool_adapter_ephemeral_bundle_delivery,
    receive_external_pool_adapter_no_work_probe_request,
};

const HOST_READY: &[u8] = b"v262.host.authenticated";
const CHILD_READY: &[u8] = b"v262.child.authenticated";
const SHUTDOWN: &[u8] = b"v262.shutdown";
const V263_CONFIG: &[u8] = br#"{"mode":"test-no-work"}"#;
const V265_CONFIG: &[u8] = br#"{"mode":"test-upstream-no-work"}"#;
const V263_CREDENTIAL: &[u8] = b"test-credential-never-production";
const V265_REQUEST: &[u8] = b"ELON-TEST-NO-WORK\n";
const V265_RESPONSE: &[u8] = b"ELON-TEST-NO-TASK\n";
const V265_PROBE_TIMEOUT: Duration = Duration::from_millis(15_000);

#[test]
#[ignore = "requires delegated cgroup v2 root execution and the static V262 fixture"]
fn linux_kernel_exec_child_completes_mutual_authentication_and_frames() {
    let parent_path = delegated_cgroup_parent_path();
    let parent = delegated_cgroup_parent(&parent_path);
    let prepared = prepare_external_pool_adapter_supervisor_session(roots())
        .expect("prepare V262 authenticated runtime");
    let (host, child_bootstrap) = prepared.split();
    let mut child = launch_materialized_fixture(&parent, child_bootstrap);
    let pid = child.pid_for_test();
    let cgroup_path = child_cgroup_path(&parent_path, &child);
    let scratch_path = child.scratch_path_for_test().to_path_buf();

    let mut session = host.authenticate().expect("authenticate post-exec host");
    session
        .send(ExternalPoolAdapterSessionFrameKind::Control, HOST_READY)
        .expect("send authenticated host-ready frame");
    let frame = session
        .receive()
        .expect("receive authenticated child-ready frame");
    assert!(frame.kind() == ExternalPoolAdapterSessionFrameKind::Control);
    assert_eq!(frame.payload(), CHILD_READY);
    assert_eq!(open_fds(pid), BTreeSet::from([0, 1, 2, 3]));
    assert_eq!(
        fs::read_to_string(cgroup_path.join("cgroup.procs"))
            .expect("read V262 cgroup membership")
            .trim(),
        pid.to_string()
    );

    session
        .send(ExternalPoolAdapterSessionFrameKind::Control, SHUTDOWN)
        .expect("send authenticated shutdown frame");
    drop(session);
    let exit = child
        .wait(Duration::from_secs(2))
        .expect("wait for authenticated runtime")
        .expect("authenticated runtime exited");
    assert_eq!(exit.exit_code, Some(0));
    assert_eq!(exit.signal, None);
    assert!(!cgroup_path.exists());
    assert!(!scratch_path.exists());
}

#[test]
#[ignore = "requires delegated cgroup v2 root execution and the static V262 fixture"]
fn linux_kernel_exec_root_drift_fails_closed_and_cleans_runtime() {
    let parent_path = delegated_cgroup_parent_path();
    let parent = delegated_cgroup_parent(&parent_path);
    let prepared = prepare_external_pool_adapter_supervisor_session(roots())
        .expect("prepare V262 drift fixture");
    let (host, mut child_bootstrap) = prepared.split();
    child_bootstrap.replace_root_argument_for_test(1, digest(0xaa));
    let mut child = launch_materialized_fixture(&parent, child_bootstrap);
    let cgroup_path = child_cgroup_path(&parent_path, &child);
    let scratch_path = child.scratch_path_for_test().to_path_buf();

    assert!(host.authenticate().is_err());
    let exit = child
        .wait(Duration::from_secs(2))
        .expect("wait for rejected runtime")
        .expect("rejected runtime exited");
    assert_eq!(exit.exit_code, Some(111));
    assert_eq!(exit.signal, None);
    assert!(!cgroup_path.exists());
    assert!(!scratch_path.exists());
}

#[test]
#[ignore = "requires delegated cgroup v2 root execution and the static V262/V263 fixture"]
fn linux_kernel_exec_child_receives_exact_ephemeral_bundle_and_zeroizes_on_shutdown() {
    let parent_path = delegated_cgroup_parent_path();
    let parent = delegated_cgroup_parent(&parent_path);
    let delivery =
        prepare_external_pool_adapter_ephemeral_bundle_delivery(263, V263_CONFIG, V263_CREDENTIAL)
            .expect("prepare V263 delivery");
    let bundle_root = delivery.bundle_root_hex();
    let prepared =
        prepare_external_pool_adapter_supervisor_session(roots_with_bundle(&bundle_root))
            .expect("prepare V263 authenticated runtime");
    let (host, child_bootstrap) = prepared.split();
    let mut child = launch_materialized_fixture(&parent, child_bootstrap);
    let pid = child.pid_for_test();
    let cgroup_path = child_cgroup_path(&parent_path, &child);
    let scratch_path = child.scratch_path_for_test().to_path_buf();

    let mut session = host.authenticate().expect("authenticate V263 host");
    let receipt = delivery
        .deliver(&mut session, &bundle_root, V263_CONFIG, V263_CREDENTIAL)
        .expect("deliver V263 ephemeral bundle");
    assert_eq!(open_fds(pid), BTreeSet::from([0, 1, 2, 3]));
    assert_eq!(
        fs::read_to_string(cgroup_path.join("cgroup.procs"))
            .expect("read V263 cgroup membership")
            .trim(),
        pid.to_string()
    );
    receipt
        .shutdown(&mut session)
        .expect("zeroize V263 delivery and shutdown");
    drop(session);
    let exit = child
        .wait(Duration::from_secs(2))
        .expect("wait for V263 runtime")
        .expect("V263 runtime exited");
    assert_eq!(exit.exit_code, Some(0));
    assert_eq!(exit.signal, None);
    assert!(!cgroup_path.exists());
    assert!(!scratch_path.exists());
}

#[test]
#[ignore = "requires delegated cgroup v2 root execution and the static V262/V263 fixture"]
fn linux_kernel_ephemeral_bundle_root_drift_fails_closed_and_cleans_runtime() {
    let parent_path = delegated_cgroup_parent_path();
    let parent = delegated_cgroup_parent(&parent_path);
    let delivery =
        prepare_external_pool_adapter_ephemeral_bundle_delivery(264, V263_CONFIG, V263_CREDENTIAL)
            .expect("prepare V263 drift delivery");
    let exact_root = delivery.bundle_root_hex();
    let prepared =
        prepare_external_pool_adapter_supervisor_session(roots_with_bundle(&digest(0xaa)))
            .expect("prepare V263 drift runtime");
    let (host, child_bootstrap) = prepared.split();
    let mut child = launch_materialized_fixture(&parent, child_bootstrap);
    let cgroup_path = child_cgroup_path(&parent_path, &child);
    let scratch_path = child.scratch_path_for_test().to_path_buf();

    let mut session = host.authenticate().expect("authenticate V263 drift host");
    assert!(delivery
        .deliver(&mut session, &exact_root, V263_CONFIG, V263_CREDENTIAL,)
        .is_err());
    drop(session);
    let exit = child
        .wait(Duration::from_secs(2))
        .expect("wait for rejected V263 runtime")
        .expect("rejected V263 runtime exited");
    assert_eq!(exit.exit_code, Some(111));
    assert_eq!(exit.signal, None);
    assert!(!cgroup_path.exists());
    assert!(!scratch_path.exists());
}

#[test]
#[ignore = "requires delegated cgroup v2 root execution and the static V265 fixture"]
fn linux_kernel_exec_child_completes_authenticated_no_work_probe_and_reaps() {
    let parent_path = delegated_cgroup_parent_path();
    let parent = delegated_cgroup_parent(&parent_path);
    let delivery =
        prepare_external_pool_adapter_ephemeral_bundle_delivery(265, V265_CONFIG, V263_CREDENTIAL)
            .expect("prepare V265 delivery");
    let bundle_root = delivery.bundle_root_hex();
    let prepared =
        prepare_external_pool_adapter_supervisor_session(roots_with_bundle(&bundle_root))
            .expect("prepare V265 authenticated runtime");
    let (host, child_bootstrap) = prepared.split();
    let mut child = launch_materialized_fixture(&parent, child_bootstrap);
    let pid = child.pid_for_test();
    let cgroup_path = child_cgroup_path(&parent_path, &child);
    let scratch_path = child.scratch_path_for_test().to_path_buf();

    let mut session = host.authenticate().expect("authenticate V265 host");
    let delivery_receipt = delivery
        .deliver(&mut session, &bundle_root, V265_CONFIG, V263_CREDENTIAL)
        .expect("deliver V265 ephemeral bundle");
    let request =
        receive_external_pool_adapter_no_work_probe_request(&mut session, V265_PROBE_TIMEOUT)
            .expect("receive V265 child-generated request");
    assert_eq!(request.request(), V265_REQUEST);
    assert_eq!(request.expected_response_bytes(), V265_RESPONSE.len());
    let probe_receipt = request
        .complete(&mut session, V265_RESPONSE)
        .expect("complete V265 child-validated response");
    assert_eq!(probe_receipt.request_bytes(), V265_REQUEST.len() as u32);
    assert_eq!(probe_receipt.response_bytes(), V265_RESPONSE.len() as u32);
    assert_eq!(open_fds(pid), BTreeSet::from([0, 1, 2, 3]));
    delivery_receipt
        .shutdown(&mut session)
        .expect("zeroize and shutdown V265 child");
    drop(session);
    let exit = child
        .wait(Duration::from_secs(2))
        .expect("wait for V265 runtime")
        .expect("V265 runtime exited");
    assert_eq!(exit.exit_code, Some(0));
    assert_eq!(exit.signal, None);
    assert!(!cgroup_path.exists());
    assert!(!scratch_path.exists());
}

fn roots() -> elon_external_pool_adapter_session_core::ExternalPoolAdapterSessionRoots {
    roots_with_bundle(&digest(0x66))
}

fn roots_with_bundle(
    bundle_root: &str,
) -> elon_external_pool_adapter_session_core::ExternalPoolAdapterSessionRoots {
    external_pool_adapter_session_roots(
        &digest(0x11),
        &digest(0x22),
        &digest(0x33),
        &digest(0x44),
        bundle_root,
    )
    .expect("construct V262 fixture roots")
}

fn digest(byte: u8) -> String {
    format!("{byte:02x}").repeat(32)
}

fn delegated_cgroup_parent_path() -> PathBuf {
    PathBuf::from(
        std::env::var_os("ELON_V261_CGROUP_PARENT")
            .expect("ELON_V261_CGROUP_PARENT must identify a delegated test cgroup"),
    )
}

fn delegated_cgroup_parent(path: &Path) -> ExternalPoolAdapterSupervisorCgroupParent {
    ExternalPoolAdapterSupervisorCgroupParent::from_directory(
        File::open(path).expect("open delegated V262 cgroup parent"),
    )
    .expect("validate delegated V262 cgroup parent")
}

fn launch_materialized_fixture(
    parent: &ExternalPoolAdapterSupervisorCgroupParent,
    child_bootstrap: ExternalPoolAdapterChildBootstrap,
) -> ExternalPoolAdapterSupervisorChild {
    let path = PathBuf::from(
        std::env::var_os("ELON_V262_SESSION_FIXTURE")
            .expect("ELON_V262_SESSION_FIXTURE must identify the static fixture binary"),
    );
    let bytes = fs::read(path).expect("read V262 fixture binary");
    with_materialized_external_pool_adapter_test_capsule(&bytes, |capsule| {
        launch_external_pool_adapter_supervisor_child(parent, child_bootstrap, capsule)
    })
    .expect("production materializer should derive and launch the V262 fixture capsule")
}

fn child_cgroup_path(parent: &Path, child: &super::ExternalPoolAdapterSupervisorChild) -> PathBuf {
    parent.join(
        child
            .cgroup_for_test()
            .name()
            .to_str()
            .expect("UTF-8 cgroup leaf"),
    )
}

fn open_fds(pid: libc::pid_t) -> BTreeSet<u32> {
    fs::read_dir(format!("/proc/{pid}/fd"))
        .expect("list V262 child descriptors")
        .map(|entry| {
            entry
                .expect("read V262 descriptor entry")
                .file_name()
                .to_string_lossy()
                .parse::<u32>()
                .expect("numeric V262 descriptor")
        })
        .collect()
}
