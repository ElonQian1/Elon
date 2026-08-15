use std::{
    collections::BTreeSet,
    fs::{self, File},
    os::{fd::AsRawFd, unix::fs::PermissionsExt},
    path::{Path, PathBuf},
    time::Duration,
};

use super::{
    launch_external_pool_adapter_supervisor_child, ExternalPoolAdapterSupervisorCgroupParent,
    ExternalPoolAdapterSupervisorChild,
};
use crate::compute_federation::external_pool_adapter_supervisor_session::{
    external_pool_adapter_session_roots, prepare_external_pool_adapter_supervisor_session,
    ExternalPoolAdapterChildBootstrap, ExternalPoolAdapterSessionRoots,
};
use crate::store::compute_external_pool_adapter_runtime_bundle::with_materialized_external_pool_adapter_test_capsule;

#[path = "linux_test_capsule_fixture.rs"]
mod capsule_fixture;
use capsule_fixture::minimal_capsule;

const TMPFS_MAGIC: libc::c_long = 0x0102_1994;
const MARKER: &[u8; 8] = b"V261_OK\n";

#[cfg(target_env = "gnu")]
type RlimitResource = libc::__rlimit_resource_t;
#[cfg(not(target_env = "gnu"))]
type RlimitResource = libc::c_int;

enum TestCapsuleBehavior {
    BlockingMarker,
    NetworkProbe,
    DisallowedPollShape,
    DisallowedFcntlDup,
    AllowedDumpablePrctl,
    DisallowedPrctlOption,
    DisallowedPrctlArgument,
    DisallowedExecveatPathPointer,
}

#[test]
fn rejects_non_cgroup_parent_before_clone() {
    let root =
        std::env::temp_dir().join(format!("elon-v261-invalid-cgroup-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).expect("create ordinary directory");
    let directory = File::open(&root).expect("open ordinary directory");
    assert!(ExternalPoolAdapterSupervisorCgroupParent::from_directory(directory).is_err());
    fs::remove_dir(&root).expect("remove ordinary directory");
}

#[test]
#[ignore = "requires an explicitly delegated cgroup v2 parent and root execution"]
fn linux_kernel_enforces_clone3_cgroup_namespaces_root_fd_and_limits() {
    let parent_path = delegated_cgroup_parent_path();
    let parent = delegated_cgroup_parent(&parent_path);
    let prepared = prepare_external_pool_adapter_supervisor_session(roots())
        .expect("prepare V260 descriptors for V261 launch");
    let (host, child_bootstrap) = prepared.split();
    let mut child = launch_materialized_fixture(
        &parent,
        child_bootstrap,
        TestCapsuleBehavior::BlockingMarker,
    );
    let pid = child.pid_for_test();
    let cgroup_name = child
        .cgroup_for_test()
        .name()
        .to_str()
        .expect("UTF-8 cgroup leaf")
        .to_string();
    let cgroup_path = parent_path.join(&cgroup_name);
    let scratch_path = child.scratch_path_for_test().to_path_buf();

    assert_eq!(
        receive_marker(host.socket_fd_for_supervisor_test()),
        *MARKER
    );
    assert_namespace_isolation(pid);
    assert_process_status(pid);
    assert_exact_open_fds(pid);
    assert_private_root(pid);
    assert_rlimits(pid);
    assert_eq!(
        fs::read_to_string(cgroup_path.join("cgroup.procs"))
            .expect("read child cgroup membership")
            .trim(),
        pid.to_string()
    );
    assert_eq!(read_trimmed(&cgroup_path.join("pids.max")), "1");
    assert_eq!(read_trimmed(&cgroup_path.join("memory.max")), "268435456");
    assert_eq!(read_trimmed(&cgroup_path.join("memory.swap.max")), "0");
    assert_eq!(read_trimmed(&cgroup_path.join("memory.oom.group")), "1");
    assert_eq!(read_trimmed(&cgroup_path.join("cpu.max")), "100000 100000");
    assert!(scratch_path.exists());

    send_one(host.socket_fd_for_supervisor_test(), 1);
    let exit = child
        .wait(Duration::from_secs(2))
        .expect("wait by pidfd")
        .expect("child exited");
    assert_eq!(exit.exit_code, Some(0));
    assert_eq!(exit.signal, None);
    assert!(!cgroup_path.exists());
    assert!(!scratch_path.exists());
}

#[test]
#[ignore = "requires an explicitly delegated cgroup v2 parent and root execution"]
fn linux_kernel_seccomp_kills_network_syscall() {
    let parent_path = delegated_cgroup_parent_path();
    let parent = delegated_cgroup_parent(&parent_path);
    let prepared = prepare_external_pool_adapter_supervisor_session(roots())
        .expect("prepare V260 descriptors for seccomp fixture");
    let (_host, child_bootstrap) = prepared.split();
    let mut child =
        launch_materialized_fixture(&parent, child_bootstrap, TestCapsuleBehavior::NetworkProbe);
    let exit = child
        .wait(Duration::from_secs(2))
        .expect("wait for seccomp termination")
        .expect("seccomp fixture terminated");
    assert_eq!(exit.exit_code, None);
    assert_eq!(exit.signal, Some(libc::SIGSYS));
}

#[test]
#[ignore = "requires an explicitly delegated cgroup v2 parent and root execution"]
fn linux_kernel_seccomp_rejects_unapproved_poll_shape() {
    let parent = delegated_cgroup_parent(&delegated_cgroup_parent_path());
    let prepared = prepare_external_pool_adapter_supervisor_session(roots())
        .expect("prepare disallowed poll fixture");
    let (_host, child_bootstrap) = prepared.split();
    let mut child = launch_materialized_fixture(
        &parent,
        child_bootstrap,
        TestCapsuleBehavior::DisallowedPollShape,
    );
    let exit = child
        .wait(Duration::from_secs(2))
        .expect("wait for disallowed poll termination")
        .expect("disallowed poll fixture terminated");
    assert_eq!(exit.exit_code, None);
    assert_eq!(exit.signal, Some(libc::SIGSYS));
}

#[test]
#[ignore = "requires an explicitly delegated cgroup v2 parent and root execution"]
fn linux_kernel_seccomp_rejects_fcntl_descriptor_duplication() {
    let parent = delegated_cgroup_parent(&delegated_cgroup_parent_path());
    let prepared = prepare_external_pool_adapter_supervisor_session(roots())
        .expect("prepare disallowed fcntl fixture");
    let (_host, child_bootstrap) = prepared.split();
    let mut child = launch_materialized_fixture(
        &parent,
        child_bootstrap,
        TestCapsuleBehavior::DisallowedFcntlDup,
    );
    let exit = child
        .wait(Duration::from_secs(2))
        .expect("wait for disallowed fcntl termination")
        .expect("disallowed fcntl fixture terminated");
    assert_eq!(exit.exit_code, None);
    assert_eq!(exit.signal, Some(libc::SIGSYS));
}

#[test]
#[ignore = "requires an explicitly delegated cgroup v2 parent and root execution"]
fn linux_kernel_seccomp_allows_exact_dumpable_prctl_shapes() {
    let parent = delegated_cgroup_parent(&delegated_cgroup_parent_path());
    let prepared = prepare_external_pool_adapter_supervisor_session(roots())
        .expect("prepare exact dumpable prctl fixture");
    let (_host, child_bootstrap) = prepared.split();
    let mut child = launch_materialized_fixture(
        &parent,
        child_bootstrap,
        TestCapsuleBehavior::AllowedDumpablePrctl,
    );
    let exit = child
        .wait(Duration::from_secs(2))
        .expect("wait for exact dumpable prctl fixture")
        .expect("exact dumpable prctl fixture terminated");
    assert_eq!(exit.exit_code, Some(0));
    assert_eq!(exit.signal, None);
}

#[test]
#[ignore = "requires an explicitly delegated cgroup v2 parent and root execution"]
fn linux_kernel_seccomp_rejects_execveat_pointer_and_other_prctl_shapes() {
    for (behavior, label) in [
        (
            TestCapsuleBehavior::DisallowedExecveatPathPointer,
            "alternate execveat pathname pointer",
        ),
        (
            TestCapsuleBehavior::DisallowedPrctlOption,
            "unapproved prctl option",
        ),
        (
            TestCapsuleBehavior::DisallowedPrctlArgument,
            "nonzero prctl argument",
        ),
    ] {
        let parent = delegated_cgroup_parent(&delegated_cgroup_parent_path());
        let prepared = prepare_external_pool_adapter_supervisor_session(roots())
            .expect("prepare rejected exact-seccomp fixture");
        let (_host, child_bootstrap) = prepared.split();
        let mut child = launch_materialized_fixture(&parent, child_bootstrap, behavior);
        let exit = child
            .wait(Duration::from_secs(2))
            .expect("wait for rejected exact-seccomp fixture")
            .expect("rejected exact-seccomp fixture terminated");
        assert_eq!(exit.exit_code, None, "{label}");
        assert_eq!(exit.signal, Some(libc::SIGSYS), "{label}");
    }
}

#[test]
#[ignore = "requires an explicitly delegated cgroup v2 parent and root execution"]
fn linux_kernel_pidfd_termination_reaps_and_cleans() {
    let parent_path = delegated_cgroup_parent_path();
    let parent = delegated_cgroup_parent(&parent_path);
    let prepared = prepare_external_pool_adapter_supervisor_session(roots())
        .expect("prepare V260 descriptors for termination fixture");
    let (host, child_bootstrap) = prepared.split();
    let mut child = launch_materialized_fixture(
        &parent,
        child_bootstrap,
        TestCapsuleBehavior::BlockingMarker,
    );
    let cgroup_path = parent_path.join(
        child
            .cgroup_for_test()
            .name()
            .to_str()
            .expect("UTF-8 cgroup leaf"),
    );
    let scratch_path = child.scratch_path_for_test().to_path_buf();
    assert_eq!(
        receive_marker(host.socket_fd_for_supervisor_test()),
        *MARKER
    );

    let exit = child.terminate().expect("terminate child by pidfd");
    assert_eq!(exit.exit_code, None);
    assert_eq!(exit.signal, Some(libc::SIGTERM));
    assert!(!cgroup_path.exists());
    assert!(!scratch_path.exists());
}

fn delegated_cgroup_parent_path() -> PathBuf {
    let path = std::env::var_os("ELON_V261_CGROUP_PARENT")
        .expect("ELON_V261_CGROUP_PARENT must identify an explicit delegated test cgroup");
    PathBuf::from(path)
}

fn delegated_cgroup_parent(path: &Path) -> ExternalPoolAdapterSupervisorCgroupParent {
    ExternalPoolAdapterSupervisorCgroupParent::from_directory(
        File::open(path).expect("open delegated cgroup parent"),
    )
    .expect("validate delegated cgroup parent")
}

fn launch_materialized_fixture(
    parent: &ExternalPoolAdapterSupervisorCgroupParent,
    child_bootstrap: ExternalPoolAdapterChildBootstrap,
    behavior: TestCapsuleBehavior,
) -> ExternalPoolAdapterSupervisorChild {
    let source = minimal_capsule(behavior);
    with_materialized_external_pool_adapter_test_capsule(&source, |capsule| {
        launch_external_pool_adapter_supervisor_child(parent, child_bootstrap, capsule)
    })
    .expect("production materializer should derive and launch the fixture capsule")
}

fn roots() -> ExternalPoolAdapterSessionRoots {
    external_pool_adapter_session_roots(
        &digest(0x11),
        &digest(0x22),
        &digest(0x33),
        &digest(0x44),
        &digest(0x55),
    )
    .expect("construct V261 fixture roots")
}

fn digest(byte: u8) -> String {
    format!("{byte:02x}").repeat(32)
}

fn receive_marker(fd: i32) -> [u8; 8] {
    let mut poll = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    assert_eq!(unsafe { libc::poll(&mut poll, 1, 2_000) }, 1);
    let mut marker = [0_u8; 8];
    assert_eq!(
        unsafe { libc::recv(fd, marker.as_mut_ptr().cast(), marker.len(), 0) },
        marker.len() as isize
    );
    marker
}

fn send_one(fd: i32, value: u8) {
    assert_eq!(
        unsafe { libc::send(fd, (&value as *const u8).cast(), 1, libc::MSG_NOSIGNAL) },
        1
    );
}

fn assert_namespace_isolation(pid: libc::pid_t) {
    for name in ["user", "mnt", "net", "ipc", "uts"] {
        assert_ne!(
            namespace_link("self", name),
            namespace_link(&pid.to_string(), name)
        );
    }
    assert_eq!(
        namespace_link("self", "pid"),
        namespace_link(&pid.to_string(), "pid")
    );
}

fn namespace_link(subject: &str, name: &str) -> PathBuf {
    fs::read_link(format!("/proc/{subject}/ns/{name}")).expect("read namespace identity")
}

fn assert_process_status(pid: libc::pid_t) {
    let status = fs::read_to_string(format!("/proc/{pid}/status")).expect("read child status");
    assert_status_value(&status, "NoNewPrivs", "1");
    assert_status_value(&status, "Seccomp", "2");
    for capability in ["CapInh", "CapPrm", "CapEff", "CapBnd", "CapAmb"] {
        assert_status_value(&status, capability, "0000000000000000");
    }
}

fn assert_status_value(status: &str, key: &str, expected: &str) {
    let value = status
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}:")))
        .expect("status field exists")
        .trim();
    assert_eq!(value, expected);
}

fn assert_exact_open_fds(pid: libc::pid_t) {
    let observed: BTreeSet<u32> = fs::read_dir(format!("/proc/{pid}/fd"))
        .expect("list child descriptors")
        .map(|entry| {
            entry
                .expect("read descriptor entry")
                .file_name()
                .to_string_lossy()
                .parse::<u32>()
                .expect("numeric descriptor")
        })
        .collect();
    assert_eq!(observed, BTreeSet::from([0, 1, 2, 3]));
}

fn assert_private_root(pid: libc::pid_t) {
    let root = File::open(format!("/proc/{pid}/root")).expect("open child root");
    let mut status = std::mem::MaybeUninit::<libc::statfs>::zeroed();
    assert_eq!(
        unsafe { libc::fstatfs(root.as_raw_fd(), status.as_mut_ptr()) },
        0
    );
    assert_eq!(
        unsafe { status.assume_init() }.f_type as libc::c_long,
        TMPFS_MAGIC
    );
    for forbidden in ["proc", "sys", "dev"] {
        assert!(!Path::new(&format!("/proc/{pid}/root/{forbidden}")).exists());
    }
    let mode = fs::metadata(format!("/proc/{pid}/root/tmp"))
        .expect("inspect private tmp")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o700);
}

fn assert_rlimits(pid: libc::pid_t) {
    assert_rlimit(pid, libc::RLIMIT_CORE, 0);
    assert_rlimit(pid, libc::RLIMIT_NOFILE, 64);
    assert_rlimit(pid, libc::RLIMIT_NPROC, 1);
    assert_rlimit(pid, libc::RLIMIT_AS, 268_435_456);
    assert_rlimit(pid, libc::RLIMIT_FSIZE, 67_108_864);
    assert_rlimit(pid, libc::RLIMIT_STACK, 8_388_608);
    assert_rlimit(pid, libc::RLIMIT_MEMLOCK, 0);
    assert_rlimit(pid, libc::RLIMIT_CPU, 30);
}

fn assert_rlimit(pid: libc::pid_t, resource: RlimitResource, expected: u64) {
    let mut observed = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    assert_eq!(
        unsafe { libc::prlimit(pid, resource, std::ptr::null(), &mut observed) },
        0
    );
    assert_eq!(observed.rlim_cur as u64, expected);
    assert_eq!(observed.rlim_max as u64, expected);
}

fn read_trimmed(path: &Path) -> String {
    fs::read_to_string(path)
        .expect("read cgroup control")
        .trim()
        .to_string()
}
