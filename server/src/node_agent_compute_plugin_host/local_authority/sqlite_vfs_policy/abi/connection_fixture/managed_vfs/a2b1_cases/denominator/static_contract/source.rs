#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ProductionOwner {
    SqliteVfsAbiTable,
    AbiBoundary,
    AbiIoShm,
    AbiFileState,
    AbiRawState,
    AbiResultCodes,
    RegistryAbiFile,
    RegistryOperations,
    RegistryFileCustody,
    RegistryProcessOwner,
    RegistryProcessLifecycle,
    RegistryOwner,
    RegistryOwnerLifecycle,
    RegistryState,
    ManagedNamespace,
    ManagedNamespaceTypes,
    ManagedFsRoot,
    ManagedWindowsPlatform,
    ManagedShmRoot,
    ManagedCoordinator,
    ManagedTypes,
    ManagedInitialization,
    ManagedFailureCustody,
    ManagedMapping,
    ManagedLocking,
    ManagedNamespaceIo,
    ManagedNamespaceClose,
    WindowsShm,
    WindowsLocking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SourceWitness {
    pub(super) owner: ProductionOwner,
    pub(super) symbol: &'static str,
    pub(super) needle: &'static str,
    pub(super) occurrence: u8,
}

pub(super) const fn witness(
    owner: ProductionOwner,
    symbol: &'static str,
    needle: &'static str,
    occurrence: u8,
) -> SourceWitness {
    SourceWitness {
        owner,
        symbol,
        needle,
        occurrence,
    }
}

macro_rules! source {
    ($path:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path))
    };
}

pub(super) fn source_content(owner: ProductionOwner) -> &'static str {
    match owner {
        ProductionOwner::SqliteVfsAbiTable => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi.rs"),
        ProductionOwner::AbiBoundary => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/boundary.rs"),
        ProductionOwner::AbiIoShm => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/io_shm.rs"),
        ProductionOwner::AbiFileState => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/file_state.rs"),
        ProductionOwner::AbiRawState => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/raw_state.rs"),
        ProductionOwner::AbiResultCodes => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/result_codes.rs"),
        ProductionOwner::RegistryAbiFile => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/abi.rs"),
        ProductionOwner::RegistryOperations => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody/operations.rs"),
        ProductionOwner::RegistryFileCustody => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/file_custody.rs"),
        ProductionOwner::RegistryProcessOwner => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner.rs"),
        ProductionOwner::RegistryProcessLifecycle => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner/lifecycle.rs"),
        ProductionOwner::RegistryOwner => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner.rs"),
        ProductionOwner::RegistryOwnerLifecycle => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner/lifecycle.rs"),
        ProductionOwner::RegistryState => source!("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/state.rs"),
        ProductionOwner::ManagedNamespace => source!("src/node_agent_managed_fs/sqlite_namespace.rs"),
        ProductionOwner::ManagedNamespaceTypes => source!("src/node_agent_managed_fs/sqlite_namespace_types.rs"),
        ProductionOwner::ManagedFsRoot => source!("src/node_agent_managed_fs.rs"),
        ProductionOwner::ManagedWindowsPlatform => source!("src/node_agent_managed_fs/windows.rs"),
        ProductionOwner::ManagedShmRoot => source!("src/node_agent_managed_fs/sqlite_namespace_shm.rs"),
        ProductionOwner::ManagedCoordinator => source!("src/node_agent_managed_fs/sqlite_namespace_shm/coordinator.rs"),
        ProductionOwner::ManagedTypes => source!("src/node_agent_managed_fs/sqlite_namespace_shm/types.rs"),
        ProductionOwner::ManagedInitialization => source!("src/node_agent_managed_fs/sqlite_namespace_shm/node_initialization.rs"),
        ProductionOwner::ManagedFailureCustody => source!("src/node_agent_managed_fs/sqlite_namespace_shm/failure_custody.rs"),
        ProductionOwner::ManagedMapping => source!("src/node_agent_managed_fs/sqlite_namespace_shm/mapping.rs"),
        ProductionOwner::ManagedLocking => source!("src/node_agent_managed_fs/sqlite_namespace_shm/locking.rs"),
        ProductionOwner::ManagedNamespaceIo => source!("src/node_agent_managed_fs/sqlite_namespace_io.rs"),
        ProductionOwner::ManagedNamespaceClose => source!("src/node_agent_managed_fs/sqlite_namespace_close.rs"),
        ProductionOwner::WindowsShm => source!("src/node_agent_managed_fs/windows_sqlite_shm.rs"),
        ProductionOwner::WindowsLocking => source!("src/node_agent_managed_fs/windows_sqlite_locking.rs"),
    }
}

pub(super) fn validate_witness(witness: SourceWitness) -> Result<(), String> {
    if witness.symbol.is_empty() || witness.needle.is_empty() || witness.occurrence == 0 {
        return Err("source witness contains an empty identity or zero occurrence".to_owned());
    }
    let source = source_content(witness.owner);
    let span = symbol_span(source, witness.symbol).ok_or_else(|| {
        format!(
            "source witness symbol is absent or ambiguous: {}",
            witness.symbol
        )
    })?;
    if span
        .match_indices(witness.needle)
        .nth(usize::from(witness.occurrence - 1))
        .is_none()
    {
        return Err(format!(
            "source witness occurrence is absent from symbol span {}: {}",
            witness.symbol, witness.needle
        ));
    }
    Ok(())
}

const NEXT_FUNCTION_PREFIXES: &[&str] = &[
    "\nfn ",
    "\nunsafe fn ",
    "\nunsafe extern ",
    "\npub fn ",
    "\npub(super) fn ",
    "\npub(crate) fn ",
    "\npub unsafe fn ",
    "\npub(super) unsafe fn ",
    "\npub(crate) unsafe fn ",
    "\npub unsafe extern ",
    "\npub(super) unsafe extern ",
    "\npub(crate) unsafe extern ",
    "\n    fn ",
    "\n    unsafe fn ",
    "\n    unsafe extern ",
    "\n    pub fn ",
    "\n    pub(super) fn ",
    "\n    pub(crate) fn ",
    "\n    pub unsafe fn ",
    "\n    pub(super) unsafe fn ",
    "\n    pub(crate) unsafe fn ",
    "\n    pub unsafe extern ",
    "\n    pub(super) unsafe extern ",
    "\n    pub(crate) unsafe extern ",
    "\n    pub(in ",
];

fn symbol_span<'source>(source: &'source str, symbol: &str) -> Option<&'source str> {
    let mut matches = source.match_indices(symbol);
    let (start, _) = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    let after_symbol = start.checked_add(symbol.len())?;
    let tail = source.get(after_symbol..)?;
    let end = NEXT_FUNCTION_PREFIXES
        .iter()
        .filter_map(|prefix| tail.find(prefix))
        .min()
        .map_or(source.len(), |offset| after_symbol + offset);
    source.get(start..end)
}
