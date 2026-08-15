use std::collections::HashSet;

use anyhow::{bail, Result};

use super::policy::{SupervisorPolicy, CAPSULE_FD};

const AUDIT_ARCH_X86_64: u32 = 0xc000_003e;
const SECCOMP_RET_KILL_PROCESS: u32 = 0x8000_0000;
const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;
const SECCOMP_MODE_FILTER: libc::c_ulong = 2;
const SECCOMP_DATA_NR_OFFSET: u32 = 0;
const SECCOMP_DATA_ARCH_OFFSET: u32 = 4;
const SECCOMP_DATA_ARGS_OFFSET: u32 = 16;

const BPF_LD: u16 = 0x00;
const BPF_W: u16 = 0x00;
const BPF_ABS: u16 = 0x20;
const BPF_JMP: u16 = 0x05;
const BPF_JEQ: u16 = 0x10;
const BPF_JGT: u16 = 0x20;
const BPF_JSET: u16 = 0x40;
const BPF_K: u16 = 0x00;
const BPF_RET: u16 = 0x06;

pub(super) static EMPTY_EXEC_PATH: [libc::c_char; 1] = [0];

pub(super) fn build_seccomp_program(policy: &SupervisorPolicy) -> Result<Vec<libc::sock_filter>> {
    let names = &policy.confinement.seccomp.bootstrap_allowed_syscalls;
    if names.is_empty() || names.len() > 128 {
        bail!("supervisor seccomp allowlist is invalid");
    }
    let mut seen = HashSet::with_capacity(names.len());
    let mut syscalls = Vec::with_capacity(names.len());
    for name in names {
        if !seen.insert(name.as_str()) {
            bail!("supervisor seccomp allowlist contains duplicates");
        }
        syscalls.push((name.as_str(), syscall_number(name)?));
    }
    if !seen.contains("execveat")
        || !seen.contains("mmap")
        || !seen.contains("mprotect")
        || !seen.contains("fcntl")
        || !seen.contains("poll")
        || !seen.contains("prctl")
    {
        bail!("supervisor seccomp allowlist is incomplete");
    }

    let mut program = Vec::with_capacity(2 + syscalls.len() * 4 + 2);
    program.push(statement(
        BPF_LD | BPF_W | BPF_ABS,
        SECCOMP_DATA_ARCH_OFFSET,
    ));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, AUDIT_ARCH_X86_64, 1, 0));
    program.push(statement(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, SECCOMP_DATA_NR_OFFSET));
    for (name, number) in syscalls {
        match name {
            "mmap" | "mprotect" => append_no_exec_memory_rule(&mut program, number),
            "execveat" => append_execveat_rule(&mut program, number),
            "fcntl" => append_getfd_rule(&mut program, number),
            "poll" => append_bounded_poll_rule(&mut program, number),
            "prctl" => append_dumpable_prctl_rule(&mut program, number),
            _ => append_plain_allow(&mut program, number),
        }
    }
    program.push(statement(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS));
    if program.len() > 4096 || program.len() > u16::MAX as usize {
        bail!("supervisor seccomp program exceeded kernel bound");
    }
    Ok(program)
}

pub(super) unsafe fn install_seccomp_program(program: &[libc::sock_filter]) -> bool {
    if program.is_empty() || program.len() > u16::MAX as usize {
        return false;
    }
    let mut descriptor = libc::sock_fprog {
        len: program.len() as u16,
        filter: program.as_ptr().cast_mut(),
    };
    libc::prctl(
        libc::PR_SET_SECCOMP,
        SECCOMP_MODE_FILTER,
        &mut descriptor as *mut libc::sock_fprog,
    ) == 0
}

fn append_plain_allow(program: &mut Vec<libc::sock_filter>, syscall: i64) {
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, syscall as u32, 0, 1));
    program.push(statement(BPF_RET | BPF_K, SECCOMP_RET_ALLOW));
}

fn append_no_exec_memory_rule(program: &mut Vec<libc::sock_filter>, syscall: i64) {
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, syscall as u32, 0, 4));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_low_offset(2)));
    program.push(jump(
        BPF_JMP | BPF_JSET | BPF_K,
        libc::PROT_EXEC as u32,
        0,
        1,
    ));
    program.push(statement(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS));
    program.push(statement(BPF_RET | BPF_K, SECCOMP_RET_ALLOW));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, SECCOMP_DATA_NR_OFFSET));
}

fn append_execveat_rule(program: &mut Vec<libc::sock_filter>, syscall: i64) {
    let empty_path_pointer = EMPTY_EXEC_PATH.as_ptr() as usize as u64;
    let empty_path_pointer_low = empty_path_pointer as u32;
    let empty_path_pointer_high = (empty_path_pointer >> 32) as u32;

    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, syscall as u32, 0, 14));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_low_offset(0)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, CAPSULE_FD as u32, 0, 10));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_high_offset(0)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 8));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_low_offset(1)));
    program.push(jump(
        BPF_JMP | BPF_JEQ | BPF_K,
        empty_path_pointer_low,
        0,
        6,
    ));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_high_offset(1)));
    program.push(jump(
        BPF_JMP | BPF_JEQ | BPF_K,
        empty_path_pointer_high,
        0,
        4,
    ));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_low_offset(4)));
    program.push(jump(
        BPF_JMP | BPF_JEQ | BPF_K,
        libc::AT_EMPTY_PATH as u32,
        0,
        2,
    ));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_high_offset(4)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 1, 0));
    program.push(statement(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS));
    program.push(statement(BPF_RET | BPF_K, SECCOMP_RET_ALLOW));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, SECCOMP_DATA_NR_OFFSET));
}

fn append_dumpable_prctl_rule(program: &mut Vec<libc::sock_filter>, syscall: i64) {
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, syscall as u32, 0, 23));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_high_offset(0)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 19));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_low_offset(0)));
    program.push(jump(
        BPF_JMP | BPF_JEQ | BPF_K,
        libc::PR_SET_DUMPABLE as u32,
        1,
        0,
    ));
    program.push(jump(
        BPF_JMP | BPF_JEQ | BPF_K,
        libc::PR_GET_DUMPABLE as u32,
        0,
        16,
    ));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_low_offset(1)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 14));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_high_offset(1)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 12));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_low_offset(2)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 10));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_high_offset(2)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 8));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_low_offset(3)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 6));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_high_offset(3)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 4));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_low_offset(4)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 2));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_high_offset(4)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 1, 0));
    program.push(statement(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS));
    program.push(statement(BPF_RET | BPF_K, SECCOMP_RET_ALLOW));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, SECCOMP_DATA_NR_OFFSET));
}

fn append_getfd_rule(program: &mut Vec<libc::sock_filter>, syscall: i64) {
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, syscall as u32, 0, 11));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_low_offset(1)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, libc::F_GETFD as u32, 0, 7));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_high_offset(1)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 5));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_high_offset(0)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 3));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_low_offset(0)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 3, 2, 0));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 5, 1, 0));
    program.push(statement(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS));
    program.push(statement(BPF_RET | BPF_K, SECCOMP_RET_ALLOW));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, SECCOMP_DATA_NR_OFFSET));
}

fn append_bounded_poll_rule(program: &mut Vec<libc::sock_filter>, syscall: i64) {
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, syscall as u32, 0, 17));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_high_offset(1)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 13));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_low_offset(1)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 3, 2, 0));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 1, 5, 0));
    program.push(statement(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_high_offset(2)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 7));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_low_offset(2)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 6, 5));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_high_offset(2)));
    program.push(jump(BPF_JMP | BPF_JEQ | BPF_K, 0, 0, 3));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, argument_low_offset(2)));
    program.push(jump(BPF_JMP | BPF_JGT | BPF_K, 5_000, 1, 0));
    program.push(jump(BPF_JMP | BPF_JGT | BPF_K, 0, 1, 0));
    program.push(statement(BPF_RET | BPF_K, SECCOMP_RET_KILL_PROCESS));
    program.push(statement(BPF_RET | BPF_K, SECCOMP_RET_ALLOW));
    program.push(statement(BPF_LD | BPF_W | BPF_ABS, SECCOMP_DATA_NR_OFFSET));
}

const fn argument_low_offset(index: u32) -> u32 {
    SECCOMP_DATA_ARGS_OFFSET + index * 8
}

const fn argument_high_offset(index: u32) -> u32 {
    argument_low_offset(index) + 4
}

const fn statement(code: u16, value: u32) -> libc::sock_filter {
    libc::sock_filter {
        code,
        jt: 0,
        jf: 0,
        k: value,
    }
}

const fn jump(code: u16, value: u32, jt: u8, jf: u8) -> libc::sock_filter {
    libc::sock_filter {
        code,
        jt,
        jf,
        k: value,
    }
}

fn syscall_number(name: &str) -> Result<i64> {
    let number = match name {
        "read" => libc::SYS_read,
        "write" => libc::SYS_write,
        "close" => libc::SYS_close,
        "fcntl" => libc::SYS_fcntl,
        "poll" => libc::SYS_poll,
        "recvmsg" => libc::SYS_recvmsg,
        "sendmsg" => libc::SYS_sendmsg,
        "exit" => libc::SYS_exit,
        "exit_group" => libc::SYS_exit_group,
        "rt_sigaction" => libc::SYS_rt_sigaction,
        "rt_sigprocmask" => libc::SYS_rt_sigprocmask,
        "rt_sigreturn" => libc::SYS_rt_sigreturn,
        "sigaltstack" => libc::SYS_sigaltstack,
        "brk" => libc::SYS_brk,
        "mmap" => libc::SYS_mmap,
        "mprotect" => libc::SYS_mprotect,
        "munmap" => libc::SYS_munmap,
        "madvise" => libc::SYS_madvise,
        "futex" => libc::SYS_futex,
        "clock_gettime" => libc::SYS_clock_gettime,
        "arch_prctl" => libc::SYS_arch_prctl,
        "set_tid_address" => libc::SYS_set_tid_address,
        "set_robust_list" => libc::SYS_set_robust_list,
        "rseq" => libc::SYS_rseq,
        "getrandom" => libc::SYS_getrandom,
        "getpid" => libc::SYS_getpid,
        "gettid" => libc::SYS_gettid,
        "prlimit64" => libc::SYS_prlimit64,
        "prctl" => libc::SYS_prctl,
        "execveat" => libc::SYS_execveat,
        _ => bail!("unsupported supervisor syscall policy entry"),
    };
    Ok(number as i64)
}
