use std::{
    collections::BTreeSet,
    ffi::CString,
    fs::{self, File},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{fs::FileExt, fs::PermissionsExt},
    },
    path::{Path, PathBuf},
    time::Duration,
};

use super::{
    launch_external_pool_adapter_supervisor_child, ExternalPoolAdapterSupervisorCapsule,
    ExternalPoolAdapterSupervisorCgroupParent,
};
use crate::compute_federation::external_pool_adapter_supervisor_session::{
    prepare_external_pool_adapter_supervisor_session, ExternalPoolAdapterSessionRoots,
};

const REQUIRED_CAPSULE_SEALS: libc::c_int =
    libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;
const TMPFS_MAGIC: libc::c_long = 0x0102_1994;
const MARKER: &[u8; 8] = b"V261_OK\n";

#[cfg(target_env = "gnu")]
type RlimitResource = libc::__rlimit_resource_t;
#[cfg(not(target_env = "gnu"))]
type RlimitResource = libc::c_int;

struct TestCapsule(File);

impl ExternalPoolAdapterSupervisorCapsule for TestCapsule {
    fn retained_sealed_image(&self) -> &File {
        &self.0
    }
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
    let capsule = sealed_capsule(minimal_capsule(false));
    let prepared = prepare_external_pool_adapter_supervisor_session(roots())
        .expect("prepare V260 descriptors for V261 launch");
    let (host, child_bootstrap) = prepared.split();
    let mut child =
        launch_external_pool_adapter_supervisor_child(&parent, child_bootstrap, &capsule)
            .expect("launch isolated supervisor child");
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
    let capsule = sealed_capsule(minimal_capsule(true));
    let prepared = prepare_external_pool_adapter_supervisor_session(roots())
        .expect("prepare V260 descriptors for seccomp fixture");
    let (_host, child_bootstrap) = prepared.split();
    let mut child =
        launch_external_pool_adapter_supervisor_child(&parent, child_bootstrap, &capsule)
            .expect("launch seccomp fixture");
    let exit = child
        .wait(Duration::from_secs(2))
        .expect("wait for seccomp termination")
        .expect("seccomp fixture terminated");
    assert_eq!(exit.exit_code, None);
    assert_eq!(exit.signal, Some(libc::SIGSYS));
}

#[test]
#[ignore = "requires an explicitly delegated cgroup v2 parent and root execution"]
fn linux_kernel_pidfd_termination_reaps_and_cleans() {
    let parent_path = delegated_cgroup_parent_path();
    let parent = delegated_cgroup_parent(&parent_path);
    let capsule = sealed_capsule(minimal_capsule(false));
    let prepared = prepare_external_pool_adapter_supervisor_session(roots())
        .expect("prepare V260 descriptors for termination fixture");
    let (host, child_bootstrap) = prepared.split();
    let mut child =
        launch_external_pool_adapter_supervisor_child(&parent, child_bootstrap, &capsule)
            .expect("launch termination fixture");
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

fn sealed_capsule(bytes: Vec<u8>) -> TestCapsule {
    let name = CString::new("elon-v261-test-capsule").expect("static memfd name");
    let fd =
        unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING) };
    assert!(fd >= 0);
    let file = unsafe { File::from_raw_fd(fd) };
    file.write_all_at(&bytes, 0).expect("write test capsule");
    assert_eq!(unsafe { libc::fchmod(fd, 0o500) }, 0);
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_ADD_SEALS, REQUIRED_CAPSULE_SEALS) },
        0
    );
    TestCapsule(file)
}

fn roots() -> ExternalPoolAdapterSessionRoots {
    ExternalPoolAdapterSessionRoots::new(
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

fn minimal_capsule(network_probe: bool) -> Vec<u8> {
    const ELF_HEADER_BYTES: usize = 64;
    const PROGRAM_HEADER_BYTES: usize = 56;
    const CODE_OFFSET: usize = 4096;
    const LOAD_ADDRESS: u64 = 0x0040_0000;

    let code = if network_probe {
        network_probe_code()
    } else {
        blocking_marker_code()
    };
    let mut image = vec![0_u8; CODE_OFFSET + code.len()];
    image[..4].copy_from_slice(b"\x7fELF");
    image[4] = 2;
    image[5] = 1;
    image[6] = 1;
    put_u16(&mut image, 16, 2);
    put_u16(&mut image, 18, 62);
    put_u32(&mut image, 20, 1);
    put_u64(&mut image, 24, LOAD_ADDRESS + CODE_OFFSET as u64);
    put_u64(&mut image, 32, ELF_HEADER_BYTES as u64);
    put_u16(&mut image, 52, ELF_HEADER_BYTES as u16);
    put_u16(&mut image, 54, PROGRAM_HEADER_BYTES as u16);
    put_u16(&mut image, 56, 1);

    let program = ELF_HEADER_BYTES;
    put_u32(&mut image, program, 1);
    put_u32(&mut image, program + 4, 5);
    put_u64(&mut image, program + 8, 0);
    put_u64(&mut image, program + 16, LOAD_ADDRESS);
    put_u64(&mut image, program + 24, LOAD_ADDRESS);
    let image_len = image.len() as u64;
    put_u64(&mut image, program + 32, image_len);
    put_u64(&mut image, program + 40, image_len);
    put_u64(&mut image, program + 48, 4096);
    image[CODE_OFFSET..].copy_from_slice(&code);
    image
}

fn blocking_marker_code() -> Vec<u8> {
    let mut code = seed_read_prefix();
    let seed_failure = emit_jne(&mut code);
    emit_close_seed(&mut code);
    emit_mov_eax(&mut code, 1);
    emit_mov_edi(&mut code, 3);
    let marker_reference = emit_lea_rsi_rip(&mut code);
    emit_mov_edx(&mut code, MARKER.len() as u32);
    emit_syscall(&mut code);
    code.extend_from_slice(&[0x83, 0xf8, MARKER.len() as u8]);
    let write_failure = emit_jne(&mut code);
    let read_retry = code.len();
    emit_mov_eax(&mut code, 0);
    emit_mov_edi(&mut code, 3);
    code.extend_from_slice(&[0x48, 0x89, 0xe6]);
    emit_mov_edx(&mut code, 1);
    emit_syscall(&mut code);
    code.extend_from_slice(&[0x83, 0xf8, libc::EAGAIN.wrapping_neg() as u8]);
    let read_would_block = emit_je(&mut code);
    code.extend_from_slice(&[0x83, 0xf8, 0x01]);
    let read_failure = emit_jne(&mut code);
    emit_exit(&mut code, 0);
    let failure = code.len();
    emit_exit(&mut code, 111);
    let marker = code.len();
    code.extend_from_slice(MARKER);
    patch_rel32(&mut code, seed_failure, failure);
    patch_rel32(&mut code, write_failure, failure);
    patch_rel32(&mut code, read_would_block, read_retry);
    patch_rel32(&mut code, read_failure, failure);
    patch_rel32(&mut code, marker_reference, marker);
    code
}

fn network_probe_code() -> Vec<u8> {
    let mut code = seed_read_prefix();
    let seed_failure = emit_jne(&mut code);
    emit_close_seed(&mut code);
    emit_mov_eax(&mut code, libc::SYS_socket as u32);
    emit_mov_edi(&mut code, libc::AF_INET as u32);
    emit_mov_esi(&mut code, libc::SOCK_STREAM as u32);
    code.extend_from_slice(&[0x31, 0xd2]);
    emit_syscall(&mut code);
    emit_exit(&mut code, 112);
    let failure = code.len();
    emit_exit(&mut code, 111);
    patch_rel32(&mut code, seed_failure, failure);
    code
}

fn seed_read_prefix() -> Vec<u8> {
    let mut code = vec![0x48, 0x83, 0xec, 0x28];
    emit_mov_eax(&mut code, 0);
    emit_mov_edi(&mut code, 5);
    code.extend_from_slice(&[0x48, 0x89, 0xe6]);
    emit_mov_edx(&mut code, 32);
    emit_syscall(&mut code);
    code.extend_from_slice(&[0x83, 0xf8, 0x20]);
    code
}

fn emit_close_seed(code: &mut Vec<u8>) {
    emit_mov_eax(code, libc::SYS_close as u32);
    emit_mov_edi(code, 5);
    emit_syscall(code);
}

fn emit_mov_eax(code: &mut Vec<u8>, value: u32) {
    code.push(0xb8);
    code.extend_from_slice(&value.to_le_bytes());
}

fn emit_mov_edi(code: &mut Vec<u8>, value: u32) {
    code.push(0xbf);
    code.extend_from_slice(&value.to_le_bytes());
}

fn emit_mov_esi(code: &mut Vec<u8>, value: u32) {
    code.push(0xbe);
    code.extend_from_slice(&value.to_le_bytes());
}

fn emit_mov_edx(code: &mut Vec<u8>, value: u32) {
    code.push(0xba);
    code.extend_from_slice(&value.to_le_bytes());
}

fn emit_syscall(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x0f, 0x05]);
}

fn emit_exit(code: &mut Vec<u8>, value: u32) {
    emit_mov_eax(code, libc::SYS_exit as u32);
    emit_mov_edi(code, value);
    emit_syscall(code);
}

fn emit_jne(code: &mut Vec<u8>) -> usize {
    code.extend_from_slice(&[0x0f, 0x85]);
    let displacement = code.len();
    code.extend_from_slice(&[0; 4]);
    displacement
}

fn emit_je(code: &mut Vec<u8>) -> usize {
    code.extend_from_slice(&[0x0f, 0x84]);
    let displacement = code.len();
    code.extend_from_slice(&[0; 4]);
    displacement
}

fn emit_lea_rsi_rip(code: &mut Vec<u8>) -> usize {
    code.extend_from_slice(&[0x48, 0x8d, 0x35]);
    let displacement = code.len();
    code.extend_from_slice(&[0; 4]);
    displacement
}

fn patch_rel32(code: &mut [u8], displacement: usize, target: usize) {
    let relative = i32::try_from(target as isize - (displacement + 4) as isize)
        .expect("fixture branch fits rel32");
    code[displacement..displacement + 4].copy_from_slice(&relative.to_le_bytes());
}

fn put_u16(output: &mut [u8], offset: usize, value: u16) {
    output[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(output: &mut [u8], offset: usize, value: u32) {
    output[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(output: &mut [u8], offset: usize, value: u64) {
    output[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
