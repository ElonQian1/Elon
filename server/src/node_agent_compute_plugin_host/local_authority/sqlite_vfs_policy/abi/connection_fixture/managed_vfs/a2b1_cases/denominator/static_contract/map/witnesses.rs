use super::super::source::{witness, ProductionOwner, SourceWitness};

pub(super) fn abi(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::AbiIoShm, symbol, needle, 1)
}

pub(super) fn boundary(needle: &'static str) -> SourceWitness {
    witness(
        ProductionOwner::AbiBoundary,
        "unsafe fn write_pointer_null",
        needle,
        1,
    )
}

pub(super) fn file_state(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::AbiFileState, symbol, needle, 1)
}

pub(super) fn raw(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::AbiRawState, symbol, needle, 1)
}

pub(super) fn adapter(needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::RegistryAbiFile, "fn shm_map", needle, 1)
}

pub(super) fn registry(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::RegistryOperations, symbol, needle, 1)
}

pub(super) fn process_owner(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::RegistryProcessOwner, symbol, needle, 1)
}

pub(super) fn registry_owner(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::RegistryOwner, symbol, needle, 1)
}

pub(super) fn registry_state(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::RegistryState, symbol, needle, 1)
}

pub(super) fn managed(needle: &'static str) -> SourceWitness {
    witness(
        ProductionOwner::ManagedMapping,
        "fn map_connection",
        needle,
        1,
    )
}

pub(super) fn pinned_map(needle: &'static str) -> SourceWitness {
    witness(
        ProductionOwner::ManagedMapping,
        "pub(crate) fn map(",
        needle,
        1,
    )
}

pub(super) fn managed_types(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::ManagedTypes, symbol, needle, 1)
}

pub(super) fn coordinator(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::ManagedCoordinator, symbol, needle, 1)
}

pub(super) fn windows_shm(symbol: &'static str, needle: &'static str) -> SourceWitness {
    witness(ProductionOwner::WindowsShm, symbol, needle, 1)
}

pub(super) fn abi_failure() -> SourceWitness {
    abi(
        "unsafe extern \"C\" fn map",
        "Err(()) => result_codes::SHM_MAP_UNAVAILABLE",
    )
}

pub(super) fn abi_ok() -> SourceWitness {
    abi("unsafe extern \"C\" fn map", "ffi::SQLITE_OK")
}
