use std::{fs::File, os::unix::fs::FileExt};

use super::types::ExternalPoolAdapterEntrypointCapsuleError;

const ELF_HEADER_BYTES: usize = 64;
const PROGRAM_HEADER_BYTES: usize = 56;
const MAX_PROGRAM_HEADERS: u16 = 256;
const X86_64_PAGE_BYTES: u64 = 4096;
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const ELFOSABI_SYSV: u8 = 0;
const ELFOSABI_LINUX: u8 = 3;
const ET_EXEC: u16 = 2;
const EM_X86_64: u16 = 62;
const EV_CURRENT: u32 = 1;
const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PT_INTERP: u32 = 3;
const PT_GNU_STACK: u32 = 0x6474_e551;
const PF_X: u32 = 1;
const PF_W: u32 = 2;

#[derive(Clone)]
pub(super) struct ElfProgramHeader([u8; PROGRAM_HEADER_BYTES]);

#[derive(Clone, Copy)]
pub(super) struct ElfLoad {
    pub(super) index: usize,
    pub(super) old_offset: u64,
    pub(super) virtual_address: u64,
    pub(super) file_size: u64,
    pub(super) memory_size: u64,
    pub(super) alignment: u64,
}

impl ElfProgramHeader {
    pub(super) fn empty() -> Self {
        Self([0; PROGRAM_HEADER_BYTES])
    }

    pub(super) fn bytes(&self) -> &[u8; PROGRAM_HEADER_BYTES] {
        &self.0
    }

    pub(super) fn kind(&self) -> u32 {
        u32_at(&self.0, 0)
    }

    pub(super) fn file_offset(&self) -> u64 {
        u64_at(&self.0, 8)
    }

    pub(super) fn virtual_address(&self) -> u64 {
        u64_at(&self.0, 16)
    }

    pub(super) fn file_size(&self) -> u64 {
        u64_at(&self.0, 32)
    }

    pub(super) fn alignment(&self) -> u64 {
        u64_at(&self.0, 48)
    }

    pub(super) fn set_kind(&mut self, value: u32) {
        put_u32(&mut self.0, 0, value);
    }

    pub(super) fn set_flags(&mut self, value: u32) {
        put_u32(&mut self.0, 4, value);
    }

    pub(super) fn set_file_offset(&mut self, value: u64) {
        put_u64(&mut self.0, 8, value);
    }

    pub(super) fn set_virtual_address(&mut self, value: u64) {
        put_u64(&mut self.0, 16, value);
    }

    pub(super) fn set_physical_address(&mut self, value: u64) {
        put_u64(&mut self.0, 24, value);
    }

    pub(super) fn set_file_size(&mut self, value: u64) {
        put_u64(&mut self.0, 32, value);
    }

    pub(super) fn set_memory_size(&mut self, value: u64) {
        put_u64(&mut self.0, 40, value);
    }

    pub(super) fn set_alignment(&mut self, value: u64) {
        put_u64(&mut self.0, 48, value);
    }
}

pub(super) fn validate_static_elf64_x86_64(
    file: &File,
    source_size: u64,
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    if source_size < ELF_HEADER_BYTES as u64 {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
    }
    let mut header = [0_u8; ELF_HEADER_BYTES];
    read_exact_at(file, &mut header, 0)?;
    if &header[..4] != b"\x7fELF"
        || header[4] != ELFCLASS64
        || header[5] != ELFDATA2LSB
        || header[6] != EV_CURRENT as u8
        || !matches!(header[7], ELFOSABI_SYSV | ELFOSABI_LINUX)
        || header[8] != 0
        || header[9..16].iter().any(|byte| *byte != 0)
        || u16_at(&header, 16) != ET_EXEC
        || u16_at(&header, 18) != EM_X86_64
        || u32_at(&header, 20) != EV_CURRENT
        || u16_at(&header, 52) != ELF_HEADER_BYTES as u16
        || u16_at(&header, 54) != PROGRAM_HEADER_BYTES as u16
    {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
    }
    let entry = u64_at(&header, 24);
    let program_offset = u64_at(&header, 32);
    let program_count = u16_at(&header, 56);
    if program_count == 0 || program_count > MAX_PROGRAM_HEADERS {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
    }
    let program_bytes = u64::from(program_count)
        .checked_mul(PROGRAM_HEADER_BYTES as u64)
        .and_then(|length| program_offset.checked_add(length))
        .filter(|end| *end <= source_size)
        .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;
    if program_offset < ELF_HEADER_BYTES as u64 || program_bytes <= program_offset {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
    }

    let mut file_ranges = Vec::with_capacity(program_count as usize);
    let mut memory_ranges = Vec::with_capacity(program_count as usize);
    let mut executable_entry = false;
    let mut executable_load = false;
    let mut previous_load_virtual_address = None;
    for index in 0..u64::from(program_count) {
        let offset = program_offset
            .checked_add(index * PROGRAM_HEADER_BYTES as u64)
            .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;
        let mut item = [0_u8; PROGRAM_HEADER_BYTES];
        read_exact_at(file, &mut item, offset)?;
        let kind = u32_at(&item, 0);
        let flags = u32_at(&item, 4);
        let segment_alignment = u64_at(&item, 48);
        if segment_alignment > 1 && !segment_alignment.is_power_of_two() {
            return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
        }
        if flags & PF_X != 0 && flags & PF_W != 0 {
            return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
        }
        if kind == PT_GNU_STACK && flags & PF_X != 0 {
            return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
        }
        if matches!(kind, PT_INTERP | PT_DYNAMIC) {
            return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
        }
        if kind != PT_LOAD {
            continue;
        }
        let file_offset = u64_at(&item, 8);
        let virtual_address = u64_at(&item, 16);
        let file_size = u64_at(&item, 32);
        let memory_size = u64_at(&item, 40);
        let alignment = u64_at(&item, 48);
        if let Some(previous) = previous_load_virtual_address {
            if virtual_address <= previous {
                return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
            }
        }
        previous_load_virtual_address = Some(virtual_address);
        if memory_size == 0
            || file_size == 0
            || memory_size < file_size
            || virtual_address % X86_64_PAGE_BYTES != file_offset % X86_64_PAGE_BYTES
            || (alignment != 0
                && (!alignment.is_power_of_two()
                    || virtual_address % alignment != file_offset % alignment))
        {
            return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
        }
        let memory_end = virtual_address
            .checked_add(memory_size)
            .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;
        let mapped_start = virtual_address & !(X86_64_PAGE_BYTES - 1);
        let mapped_end = memory_end
            .checked_add(X86_64_PAGE_BYTES - 1)
            .map(|end| end & !(X86_64_PAGE_BYTES - 1))
            .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;
        if mapped_end <= mapped_start
            || memory_ranges
                .iter()
                .any(|&(start, end)| mapped_start < end && start < mapped_end)
        {
            return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
        }
        memory_ranges.push((mapped_start, mapped_end));
        let file_end = file_offset
            .checked_add(file_size)
            .filter(|end| *end <= source_size)
            .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;
        if file_ranges
            .iter()
            .any(|&(start, end)| file_offset < end && start < file_end)
        {
            return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
        }
        file_ranges.push((file_offset, file_end));
        if flags & PF_X != 0 {
            executable_load = true;
            let file_backed_end = virtual_address
                .checked_add(file_size)
                .ok_or(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable)?;
            if entry >= virtual_address && entry < file_backed_end {
                executable_entry = true;
            }
        }
    }
    if !executable_load || !executable_entry {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
    }
    Ok(())
}

pub(super) fn parse_static_elf64_x86_64(
    file: &File,
    source_size: u64,
) -> Result<
    (
        [u8; ELF_HEADER_BYTES],
        Vec<ElfProgramHeader>,
        u64,
        Vec<ElfLoad>,
    ),
    ExternalPoolAdapterEntrypointCapsuleError,
> {
    validate_static_elf64_x86_64(file, source_size)?;
    let mut header = [0_u8; ELF_HEADER_BYTES];
    read_exact_at(file, &mut header, 0)?;
    let program_offset = u64_at(&header, 32);
    let program_count = u16_at(&header, 56) as usize;
    let mut headers = Vec::with_capacity(program_count);
    let mut loads = Vec::new();
    let mut phdr_count = 0_u8;
    for index in 0..program_count {
        let mut bytes = [0_u8; PROGRAM_HEADER_BYTES];
        read_exact_at(
            file,
            &mut bytes,
            program_offset + (index * PROGRAM_HEADER_BYTES) as u64,
        )?;
        let item = ElfProgramHeader(bytes);
        if item.kind() == 6 {
            phdr_count = phdr_count.saturating_add(1);
        }
        if item.kind() == PT_LOAD {
            loads.push(ElfLoad {
                index,
                old_offset: item.file_offset(),
                virtual_address: item.virtual_address(),
                file_size: item.file_size(),
                memory_size: u64_at(item.bytes(), 40),
                alignment: item.alignment(),
            });
        }
        headers.push(item);
    }
    if phdr_count > 1 {
        return Err(ExternalPoolAdapterEntrypointCapsuleError::UnsafeExecutable);
    }
    Ok((header, headers, u64_at(&header, 24), loads))
}

fn read_exact_at(
    file: &File,
    mut output: &mut [u8],
    mut offset: u64,
) -> Result<(), ExternalPoolAdapterEntrypointCapsuleError> {
    while !output.is_empty() {
        let read = file
            .read_at(output, offset)
            .map_err(|_| ExternalPoolAdapterEntrypointCapsuleError::ContentDrift)?;
        if read == 0 {
            return Err(ExternalPoolAdapterEntrypointCapsuleError::ContentDrift);
        }
        offset = offset
            .checked_add(read as u64)
            .ok_or(ExternalPoolAdapterEntrypointCapsuleError::ContentDrift)?;
        output = &mut output[read..];
    }
    Ok(())
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(
        bytes[offset..offset + 2]
            .try_into()
            .expect("fixed ELF field"),
    )
}

fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("fixed ELF field"),
    )
}

fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        bytes[offset..offset + 8]
            .try_into()
            .expect("fixed ELF field"),
    )
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
