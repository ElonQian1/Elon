use std::{
    collections::BTreeSet,
    ffi::CString,
    fs::{self, File},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::FileExt,
    },
    path::{Path, PathBuf},
    time::Duration,
};

use super::{
    launch_external_pool_adapter_supervisor_child, ExternalPoolAdapterSupervisorCapsule,
    ExternalPoolAdapterSupervisorCgroupParent,
};
use crate::compute_federation::external_pool_adapter_supervisor_session::{
    external_pool_adapter_session_roots, prepare_external_pool_adapter_supervisor_session,
    ExternalPoolAdapterSessionFrameKind,
};

const REQUIRED_CAPSULE_SEALS: libc::c_int =
    libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;
const HOST_READY: &[u8] = b"v262.host.authenticated";
const CHILD_READY: &[u8] = b"v262.child.authenticated";
const SHUTDOWN: &[u8] = b"v262.shutdown";

struct TestCapsule(File);

impl ExternalPoolAdapterSupervisorCapsule for TestCapsule {
    fn retained_sealed_image(&self) -> &File {
        &self.0
    }
}

#[test]
#[ignore = "requires delegated cgroup v2 root execution and the static V262 fixture"]
fn linux_kernel_exec_child_completes_mutual_authentication_and_frames() {
    let parent_path = delegated_cgroup_parent_path();
    let parent = delegated_cgroup_parent(&parent_path);
    let capsule = sealed_fixture_capsule();
    let prepared = prepare_external_pool_adapter_supervisor_session(roots())
        .expect("prepare V262 authenticated runtime");
    let (host, child_bootstrap) = prepared.split();
    let mut child =
        launch_external_pool_adapter_supervisor_child(&parent, child_bootstrap, &capsule)
            .expect("launch V262 authenticated runtime");
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
    let capsule = sealed_fixture_capsule();
    let prepared = prepare_external_pool_adapter_supervisor_session(roots())
        .expect("prepare V262 drift fixture");
    let (host, mut child_bootstrap) = prepared.split();
    child_bootstrap.replace_root_argument_for_test(1, digest(0xaa));
    let mut child =
        launch_external_pool_adapter_supervisor_child(&parent, child_bootstrap, &capsule)
            .expect("launch V262 drift fixture");
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

fn roots() -> elon_external_pool_adapter_session_core::ExternalPoolAdapterSessionRoots {
    external_pool_adapter_session_roots(
        &digest(0x11),
        &digest(0x22),
        &digest(0x33),
        &digest(0x44),
        &digest(0x55),
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

fn sealed_fixture_capsule() -> TestCapsule {
    let path = PathBuf::from(
        std::env::var_os("ELON_V262_SESSION_FIXTURE")
            .expect("ELON_V262_SESSION_FIXTURE must identify the static fixture binary"),
    );
    let bytes = fs::read(path).expect("read V262 fixture binary");
    let name = CString::new("elon-v262-session-fixture").expect("static memfd name");
    let fd =
        unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING) };
    assert!(fd >= 0);
    let file = unsafe { File::from_raw_fd(fd) };
    file.write_all_at(&bytes, 0).expect("write V262 capsule");
    assert_eq!(unsafe { libc::fchmod(fd, 0o500) }, 0);
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_ADD_SEALS, REQUIRED_CAPSULE_SEALS) },
        0
    );
    TestCapsule(file)
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
