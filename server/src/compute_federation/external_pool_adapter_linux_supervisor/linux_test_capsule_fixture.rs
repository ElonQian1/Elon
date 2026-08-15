use super::{TestCapsuleBehavior, MARKER};

pub(super) fn minimal_capsule(behavior: TestCapsuleBehavior) -> Vec<u8> {
    const ELF_HEADER_BYTES: usize = 64;
    const PROGRAM_HEADER_BYTES: usize = 56;
    const CODE_OFFSET: usize = 4096;
    const LOAD_ADDRESS: u64 = 0x0040_0000;

    let code = match behavior {
        TestCapsuleBehavior::BlockingMarker => blocking_marker_code(),
        TestCapsuleBehavior::NetworkProbe => network_probe_code(),
        TestCapsuleBehavior::DisallowedPollShape => disallowed_poll_shape_code(),
        TestCapsuleBehavior::DisallowedFcntlDup => disallowed_fcntl_dup_code(),
        TestCapsuleBehavior::AllowedDumpablePrctl => allowed_dumpable_prctl_code(),
        TestCapsuleBehavior::DisallowedPrctlOption => disallowed_prctl_option_code(),
        TestCapsuleBehavior::DisallowedPrctlArgument => disallowed_prctl_argument_code(),
        TestCapsuleBehavior::DisallowedExecveatPathPointer => {
            disallowed_execveat_path_pointer_code()
        }
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

fn disallowed_poll_shape_code() -> Vec<u8> {
    let mut code = seed_read_prefix();
    let seed_failure = emit_jne(&mut code);
    emit_close_seed(&mut code);
    emit_mov_eax(&mut code, libc::SYS_poll as u32);
    emit_mov_edi(&mut code, 0);
    emit_mov_esi(&mut code, 2);
    emit_mov_edx(&mut code, 0);
    emit_syscall(&mut code);
    emit_exit(&mut code, 113);
    let failure = code.len();
    emit_exit(&mut code, 111);
    patch_rel32(&mut code, seed_failure, failure);
    code
}

fn disallowed_fcntl_dup_code() -> Vec<u8> {
    let mut code = seed_read_prefix();
    let seed_failure = emit_jne(&mut code);
    emit_close_seed(&mut code);
    emit_mov_eax(&mut code, libc::SYS_fcntl as u32);
    emit_mov_edi(&mut code, 3);
    emit_mov_esi(&mut code, libc::F_DUPFD_CLOEXEC as u32);
    emit_mov_edx(&mut code, 10);
    emit_syscall(&mut code);
    emit_exit(&mut code, 114);
    let failure = code.len();
    emit_exit(&mut code, 111);
    patch_rel32(&mut code, seed_failure, failure);
    code
}

fn allowed_dumpable_prctl_code() -> Vec<u8> {
    let mut code = seed_read_prefix();
    let seed_failure = emit_jne(&mut code);
    emit_close_seed(&mut code);
    emit_prctl(&mut code, libc::PR_SET_DUMPABLE as u32, 0);
    code.extend_from_slice(&[0x83, 0xf8, 0x00]);
    let set_failure = emit_jne(&mut code);
    emit_prctl(&mut code, libc::PR_GET_DUMPABLE as u32, 0);
    code.extend_from_slice(&[0x83, 0xf8, 0x00]);
    let get_failure = emit_jne(&mut code);
    emit_exit(&mut code, 0);
    let failure = code.len();
    emit_exit(&mut code, 111);
    patch_rel32(&mut code, seed_failure, failure);
    patch_rel32(&mut code, set_failure, failure);
    patch_rel32(&mut code, get_failure, failure);
    code
}

fn disallowed_prctl_option_code() -> Vec<u8> {
    let mut code = seed_read_prefix();
    let seed_failure = emit_jne(&mut code);
    emit_close_seed(&mut code);
    emit_prctl(&mut code, libc::PR_SET_NAME as u32, 0);
    emit_exit(&mut code, 115);
    let failure = code.len();
    emit_exit(&mut code, 111);
    patch_rel32(&mut code, seed_failure, failure);
    code
}

fn disallowed_prctl_argument_code() -> Vec<u8> {
    let mut code = seed_read_prefix();
    let seed_failure = emit_jne(&mut code);
    emit_close_seed(&mut code);
    emit_prctl(&mut code, libc::PR_GET_DUMPABLE as u32, 1);
    emit_exit(&mut code, 116);
    let failure = code.len();
    emit_exit(&mut code, 111);
    patch_rel32(&mut code, seed_failure, failure);
    code
}

fn disallowed_execveat_path_pointer_code() -> Vec<u8> {
    let mut code = seed_read_prefix();
    let seed_failure = emit_jne(&mut code);
    emit_close_seed(&mut code);
    emit_mov_eax(&mut code, libc::SYS_execveat as u32);
    emit_mov_edi(&mut code, 4);
    let empty_path_reference = emit_lea_rsi_rip(&mut code);
    emit_mov_edx(&mut code, 0);
    emit_zero_r10d(&mut code);
    emit_mov_r8d(&mut code, libc::AT_EMPTY_PATH as u32);
    emit_syscall(&mut code);
    emit_exit(&mut code, 117);
    let failure = code.len();
    emit_exit(&mut code, 111);
    let alternate_empty_path = code.len();
    code.push(0);
    patch_rel32(&mut code, seed_failure, failure);
    patch_rel32(&mut code, empty_path_reference, alternate_empty_path);
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

fn emit_prctl(code: &mut Vec<u8>, option: u32, argument: u32) {
    emit_mov_eax(code, libc::SYS_prctl as u32);
    emit_mov_edi(code, option);
    emit_mov_esi(code, argument);
    emit_mov_edx(code, 0);
    emit_zero_r10d(code);
    code.extend_from_slice(&[0x45, 0x31, 0xc0]);
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

fn emit_zero_r10d(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x45, 0x31, 0xd2]);
}

fn emit_mov_r8d(code: &mut Vec<u8>, value: u32) {
    code.extend_from_slice(&[0x41, 0xb8]);
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
