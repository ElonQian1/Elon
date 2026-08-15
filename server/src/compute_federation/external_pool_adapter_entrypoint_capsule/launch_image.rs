//! Derives the one post-exec identity-reset image from an already sealed source capsule.

use std::fs::File;

use zeroize::Zeroize;

use super::{
    elf::{parse_static_elf64_x86_64, ElfLoad, ElfProgramHeader},
    launch_image_io::{
        copy_range, create_launch_memfd, hash_exact, identity, require_launch_custody,
        require_source_custody, seal_launch, set_length, write_exact_at, MAX_IMAGE_BYTES,
    },
    types::ExternalPoolAdapterEntrypointCapsuleError,
};

const ELF_HEADER_BYTES: usize = 64;
const PROGRAM_HEADER_BYTES: usize = 56;
const MAX_PROGRAM_HEADERS: usize = 257;
const PAGE_BYTES: u64 = 4096;
const PT_LOAD: u32 = 1;
const PT_PHDR: u32 = 6;
const PF_R: u32 = 4;
const PF_X: u32 = 1;

pub(super) struct LaunchImage {
    pub(super) file: File,
    pub(super) sha256: String,
    pub(super) size_bytes: u64,
}

pub(super) fn derive(
    source: &File,
) -> Result<LaunchImage, ExternalPoolAdapterEntrypointCapsuleError> {
    let source_size = source
        .metadata()
        .map_err(|_| ExternalPoolAdapterEntrypointCapsuleError::ContentDrift)?
        .len();
    require_source_custody(source, source_size)?;
    let source_before = identity(source)?;
    let mut source_digest = hash_exact(source, source_size)?;
    let (mut elf, mut headers, original_entry, loads) =
        parse_static_elf64_x86_64(source, source_size)?;
    let new_count = headers
        .len()
        .checked_add(1)
        .filter(|count| *count <= MAX_PROGRAM_HEADERS)
        .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;
    let table_end = ELF_HEADER_BYTES
        .checked_add(new_count * PROGRAM_HEADER_BYTES)
        .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;
    let stub_offset = align_up(table_end as u64, 16)?;
    let stub = build_stub(original_entry)?;
    let stub_file_size = stub_offset
        .checked_add(stub.len() as u64)
        .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;
    let stub_memory_size = align_up(stub_file_size, PAGE_BYTES)?;
    let stub_vaddr = choose_stub_address(&loads, stub_memory_size)?;
    let stub_entry = stub_vaddr
        .checked_add(stub_offset)
        .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;

    let original_headers = headers.clone();
    let output_size = relocate_headers(
        source_size,
        &mut headers,
        &loads,
        stub_file_size,
        stub_memory_size,
    )?;
    headers.push(stub_header(stub_vaddr, stub_file_size, stub_memory_size));
    rewrite_elf_header(&mut elf, stub_entry, headers.len())?;
    rewrite_phdr_header(&mut headers, stub_vaddr)?;

    let launch = create_launch_memfd()?;
    set_length(&launch, output_size.max(stub_file_size))?;
    write_exact_at(&launch, &elf, 0)?;
    for (index, header) in headers.iter().enumerate() {
        write_exact_at(
            &launch,
            header.bytes(),
            (ELF_HEADER_BYTES + index * PROGRAM_HEADER_BYTES) as u64,
        )?;
    }
    write_exact_at(&launch, &stub, stub_offset)?;
    copy_relocated_ranges(source, &launch, &original_headers, &headers, &loads)?;

    let output_size = output_size.max(stub_file_size);
    let mut launch_digest = hash_exact(&launch, output_size)?;
    seal_launch(&launch)?;
    require_launch_custody(&launch, output_size)?;
    let launch_identity = identity(&launch)?;
    let mut sealed_digest = hash_exact(&launch, output_size)?;
    let mut source_after_digest = hash_exact(source, source_size)?;
    let source_after = identity(source)?;
    let exact = launch_digest == sealed_digest
        && source_digest == source_after_digest
        && source_before == source_after
        && launch_identity == identity(&launch)?;
    source_digest.zeroize();
    launch_digest.zeroize();
    source_after_digest.zeroize();
    if !exact {
        sealed_digest.zeroize();
        return Err(ExternalPoolAdapterEntrypointCapsuleError::ContentDrift);
    }
    Ok(LaunchImage {
        file: launch,
        sha256: hex::encode(sealed_digest),
        size_bytes: output_size,
    })
}

fn relocate_headers(
    source_size: u64,
    headers: &mut [ElfProgramHeader],
    loads: &[ElfLoad],
    stub_file_size: u64,
    stub_memory_size: u64,
) -> Result<u64, ExternalPoolAdapterEntrypointCapsuleError> {
    let originals = headers.to_vec();
    let mut cursor = align_up(stub_file_size, PAGE_BYTES)?;
    for load in loads {
        let target =
            congruent_at_or_after(cursor, load.virtual_address, load.alignment.max(PAGE_BYTES))?;
        headers[load.index].set_file_offset(target);
        cursor = align_up(
            target
                .checked_add(load.file_size)
                .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?,
            PAGE_BYTES,
        )?;
    }
    let relocated_offsets: Vec<(usize, u64)> = loads
        .iter()
        .map(|load| (load.index, headers[load.index].file_offset()))
        .collect();
    for (index, header) in headers.iter_mut().enumerate() {
        if header.kind() == PT_LOAD || header.kind() == PT_PHDR || header.file_size() == 0 {
            continue;
        }
        let old = originals[index].file_offset();
        let old_end = old
            .checked_add(header.file_size())
            .filter(|end| *end <= source_size)
            .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;
        let contained_target = loads
            .iter()
            .find(|load| {
                old >= load.old_offset
                    && load
                        .old_offset
                        .checked_add(load.file_size)
                        .is_some_and(|load_end| old_end <= load_end)
            })
            .and_then(|load| {
                relocated_offsets
                    .iter()
                    .find_map(|(index, offset)| (*index == load.index).then_some(*offset))
                    .and_then(|offset| offset.checked_add(old - load.old_offset))
            });
        let target = match contained_target {
            Some(target) => target,
            None => congruent_at_or_after(cursor, old, header.alignment().max(1))?,
        };
        header.set_file_offset(target);
        cursor = cursor.max(
            target
                .checked_add(header.file_size())
                .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?,
        );
    }
    let output_size = cursor.max(stub_file_size).max(stub_memory_size);
    if output_size > MAX_IMAGE_BYTES {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
    }
    Ok(output_size)
}

fn copy_relocated_ranges(
    source: &File,
    launch: &File,
    original_headers: &[ElfProgramHeader],
    headers: &[ElfProgramHeader],
    loads: &[ElfLoad],
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    for load in loads {
        copy_range(
            source,
            launch,
            load.old_offset,
            headers[load.index].file_offset(),
            load.file_size,
        )?;
    }
    for (index, original) in original_headers.iter().enumerate() {
        if original.kind() == PT_LOAD || original.kind() == PT_PHDR || original.file_size() == 0 {
            continue;
        }
        let original_end = original
            .file_offset()
            .checked_add(original.file_size())
            .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;
        let contained = loads.iter().any(|load| {
            original.file_offset() >= load.old_offset
                && load
                    .old_offset
                    .checked_add(load.file_size)
                    .is_some_and(|load_end| original_end <= load_end)
        });
        if !contained {
            copy_range(
                source,
                launch,
                original.file_offset(),
                headers[index].file_offset(),
                original.file_size(),
            )?;
        }
    }
    Ok(())
}

fn stub_header(vaddr: u64, file_size: u64, memory_size: u64) -> ElfProgramHeader {
    let mut header = ElfProgramHeader::empty();
    header.set_kind(PT_LOAD);
    header.set_flags(PF_R | PF_X);
    header.set_virtual_address(vaddr);
    header.set_physical_address(vaddr);
    header.set_file_size(file_size);
    header.set_memory_size(memory_size);
    header.set_alignment(PAGE_BYTES);
    header
}

fn rewrite_elf_header(
    header: &mut [u8; ELF_HEADER_BYTES],
    entry: u64,
    count: usize,
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    put_u64(header, 24, entry);
    put_u64(header, 32, ELF_HEADER_BYTES as u64);
    put_u64(header, 40, 0);
    put_u16(
        header,
        56,
        u16::try_from(count)
            .map_err(|_| ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?,
    );
    put_u16(header, 58, 0);
    put_u16(header, 60, 0);
    put_u16(header, 62, 0);
    Ok(())
}

fn rewrite_phdr_header(
    headers: &mut [ElfProgramHeader],
    stub_vaddr: u64,
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    let table_size = u64::try_from(headers.len() * PROGRAM_HEADER_BYTES)
        .map_err(|_| ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;
    for header in headers.iter_mut().filter(|header| header.kind() == PT_PHDR) {
        header.set_file_offset(ELF_HEADER_BYTES as u64);
        header.set_virtual_address(stub_vaddr + ELF_HEADER_BYTES as u64);
        header.set_physical_address(stub_vaddr + ELF_HEADER_BYTES as u64);
        header.set_file_size(table_size);
        header.set_memory_size(table_size);
        header.set_alignment(8);
    }
    Ok(())
}

fn choose_stub_address(
    loads: &[ElfLoad],
    size: u64,
) -> Result<u64, ExternalPoolAdapterEntrypointCapsuleError> {
    let end = loads
        .iter()
        .map(|load| load.virtual_address.saturating_add(load.memory_size))
        .max()
        .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;
    let address = align_up(end, 2 * 1024 * 1024)?;
    let final_end = address
        .checked_add(size)
        .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;
    if address < 0x1_0000 || final_end >= 0x0000_8000_0000_0000 {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
    }
    Ok(address)
}

fn build_stub(original_entry: u64) -> Result<Vec<u8>, ExternalPoolAdapterEntrypointCapsuleError> {
    let mut code = Vec::with_capacity(96);
    append_prctl(&mut code, libc::PR_SET_DUMPABLE as u32);
    let set_failure = append_jne(&mut code);
    append_prctl(&mut code, libc::PR_GET_DUMPABLE as u32);
    let proof_failure = append_jne(&mut code);
    code.extend([0x48, 0xb8]);
    code.extend(original_entry.to_le_bytes());
    code.extend([0xff, 0xe0]);
    let failure = code.len();
    code.extend([0xb8, 0xe7, 0x00, 0x00, 0x00, 0xbf, 0x7f, 0x00, 0x00, 0x00]);
    code.extend([0x0f, 0x05, 0x0f, 0x0b]);
    patch_rel32(&mut code, set_failure, failure)?;
    patch_rel32(&mut code, proof_failure, failure)?;
    Ok(code)
}

fn append_prctl(code: &mut Vec<u8>, option: u32) {
    code.extend([0xb8, 0x9d, 0x00, 0x00, 0x00, 0xbf]);
    code.extend(option.to_le_bytes());
    code.extend([0x31, 0xf6, 0x31, 0xd2, 0x45, 0x31, 0xd2, 0x45, 0x31, 0xc0]);
    code.extend([0x0f, 0x05, 0x85, 0xc0]);
}

fn append_jne(code: &mut Vec<u8>) -> usize {
    code.extend([0x0f, 0x85]);
    let position = code.len();
    code.extend([0; 4]);
    position
}

fn patch_rel32(
    code: &mut [u8],
    position: usize,
    target: usize,
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    let displacement = i32::try_from(target as isize - (position + 4) as isize)
        .map_err(|_| ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;
    code[position..position + 4].copy_from_slice(&displacement.to_le_bytes());
    Ok(())
}

fn align_up(value: u64, alignment: u64) -> Result<u64, ExternalPoolAdapterEntrypointCapsuleError> {
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
        .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)
}

fn congruent_at_or_after(
    cursor: u64,
    residue: u64,
    modulus: u64,
) -> Result<u64, ExternalPoolAdapterEntrypointCapsuleError> {
    let modulus = modulus.max(1);
    let cursor_residue = cursor % modulus;
    let target_residue = residue % modulus;
    let delta = if target_residue >= cursor_residue {
        target_residue - cursor_residue
    } else {
        modulus - (cursor_residue - target_residue)
    };
    cursor
        .checked_add(delta)
        .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)
}

fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
