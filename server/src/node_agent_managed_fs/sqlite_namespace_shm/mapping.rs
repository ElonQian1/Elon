use std::{io, num::NonZeroU32, ptr::NonNull};

use super::{
    coordinator::{
        ManagedSqliteShmCoordinator, ManagedSqliteShmCoordinatorState,
        PinnedManagedSqliteShmConnection,
    },
    platform_shm,
    types::{
        ManagedSqliteShmFailure, ManagedSqliteShmFailureClass, ManagedSqliteShmFailurePhase,
        ManagedSqliteShmMapMode, ManagedSqliteShmMapOutcome, ManagedSqliteShmRegionPointer,
    },
};

pub(super) struct ManagedSqliteShmRegionMapping {
    pub(super) view: Option<platform_shm::OwnedSqliteShmView>,
    pub(super) mapping: platform_shm::OwnedSqliteShmMapping,
    pub(super) logical_pointer: Option<NonNull<u8>>,
    pub(super) logical_length: usize,
    pub(super) mapped_length: usize,
    pub(super) aligned_offset: u64,
}

// SAFETY: the mapping object and view are process handles/addresses whose lifetime is serialized
// by the coordinator mutex. SQLite performs the mapped-memory accesses; Rust never dereferences
// the pointer without the VFS's unsafe boundary.
unsafe impl Send for ManagedSqliteShmRegionMapping {}

impl ManagedSqliteShmCoordinator {
    pub(super) fn map_connection(
        &self,
        connection_id: u64,
        region: u32,
        region_size: NonZeroU32,
        mode: ManagedSqliteShmMapMode,
    ) -> Result<ManagedSqliteShmMapOutcome, ManagedSqliteShmFailure> {
        self.budget
            .validate_region_size(region_size)
            .map_err(request_failure)?;
        let logical_end = self
            .budget
            .validate_logical_end(region, region_size)
            .map_err(request_failure)?;
        let granularity = platform_shm::allocation_granularity().map_err(|error| {
            ManagedSqliteShmFailure::new(
                ManagedSqliteShmFailurePhase::RequestValidation,
                classify_platform(&error),
                error,
            )
        })?;
        if granularity == 0 {
            return Err(request_failure(io::Error::new(
                io::ErrorKind::InvalidData,
                "NODE_MANAGED_SQLITE_SHM_ALLOCATION_GRANULARITY_ZERO",
            )));
        }

        let mut state = self.state.lock().map_err(|_| self.poisoned_failure())?;
        if !state.connections.contains_key(&connection_id) {
            return Err(protocol("NODE_MANAGED_SQLITE_SHM_CONNECTION_NOT_ATTACHED"));
        }
        if let Some(poison) = state.poisoned {
            return Err(poison.failure());
        }

        let (current_size, initialization_mutated) = {
            let (node, initialization_mutated) = self.ensure_node(&mut state)?;
            match node.region_size {
                Some(expected) if expected != region_size => {
                    return Err(protocol("NODE_MANAGED_SQLITE_SHM_REGION_SIZE_CHANGED"));
                }
                None => node.region_size = Some(region_size),
                Some(_) => {}
            }
            let current_size = node.file.size().map_err(|error| {
                let error = io::Error::other(error);
                ManagedSqliteShmFailure::new(
                    ManagedSqliteShmFailurePhase::FileSize,
                    mutation_class(initialization_mutated, &error),
                    error,
                )
            })?;
            (current_size, initialization_mutated)
        };
        self.budget
            .validate_existing_size(current_size)
            .map_err(request_failure)?;
        let mut file_grew = false;
        if current_size < logical_end {
            if mode == ManagedSqliteShmMapMode::Observe {
                return Ok(ManagedSqliteShmMapOutcome::NotPresent);
            }
            let grow = state
                .node
                .as_mut()
                .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_BEFORE_GROW"))?
                .file
                .truncate(logical_end);
            if let Err(error) = grow {
                self.mark_poisoned(
                    &mut state,
                    ManagedSqliteShmFailurePhase::FileGrow,
                    true,
                    false,
                );
                return Err(ManagedSqliteShmFailure::poisoned(
                    ManagedSqliteShmFailurePhase::FileGrow,
                    io::Error::other(error),
                    true,
                    false,
                ));
            }
            file_grew = true;
        }

        let node = state
            .node
            .as_mut()
            .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_BEFORE_MAP"))?;
        while node.regions.len() <= region as usize {
            let index = u32::try_from(node.regions.len()).map_err(|_| {
                request_failure(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "NODE_MANAGED_SQLITE_SHM_REGION_INDEX_OVERFLOW",
                ))
            })?;
            let region_offset = u64::from(index)
                .checked_mul(u64::from(region_size.get()))
                .ok_or_else(|| {
                    request_failure(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "NODE_MANAGED_SQLITE_SHM_REGION_OFFSET_OVERFLOW",
                    ))
                })?;
            let aligned_offset = region_offset - region_offset % granularity;
            let shift = usize::try_from(region_offset - aligned_offset).map_err(|_| {
                request_failure(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "NODE_MANAGED_SQLITE_SHM_VIEW_SHIFT_OVERFLOW",
                ))
            })?;
            let logical_length = usize::try_from(region_size.get()).map_err(|_| {
                request_failure(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "NODE_MANAGED_SQLITE_SHM_REGION_LENGTH_OVERFLOW",
                ))
            })?;
            let mapped_length = shift.checked_add(logical_length).ok_or_else(|| {
                request_failure(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "NODE_MANAGED_SQLITE_SHM_VIEW_LENGTH_OVERFLOW",
                ))
            })?;
            let mapped_total = node
                .mapped_bytes
                .checked_add(mapped_length as u64)
                .ok_or_else(|| {
                    request_failure(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "NODE_MANAGED_SQLITE_SHM_MAPPED_TOTAL_OVERFLOW",
                    ))
                })?;
            self.budget
                .validate_mapped_total(mapped_total)
                .map_err(request_failure)?;

            let mut mapping =
                platform_shm::create_mapping(&node.file.file, logical_end).map_err(|error| {
                    ManagedSqliteShmFailure::new(
                        ManagedSqliteShmFailurePhase::MappingCreate,
                        mutation_class(
                            initialization_mutated || file_grew || !node.regions.is_empty(),
                            &error,
                        ),
                        error,
                    )
                })?;
            let view = match platform_shm::map_view(&mapping, aligned_offset, mapped_length) {
                Ok(view) => view,
                Err(error) => {
                    if let Err(close_error) = mapping.close_explicit() {
                        node.regions.push(ManagedSqliteShmRegionMapping {
                            mapping,
                            view: None,
                            logical_pointer: None,
                            logical_length,
                            mapped_length,
                            aligned_offset,
                        });
                        self.mark_poisoned(
                            &mut state,
                            ManagedSqliteShmFailurePhase::MappingClose,
                            true,
                            false,
                        );
                        return Err(ManagedSqliteShmFailure::poisoned(
                            ManagedSqliteShmFailurePhase::MappingClose,
                            close_error,
                            true,
                            false,
                        ));
                    }
                    return Err(ManagedSqliteShmFailure::new(
                        ManagedSqliteShmFailurePhase::ViewMap,
                        mutation_class(
                            initialization_mutated || file_grew || !node.regions.is_empty(),
                            &error,
                        ),
                        error,
                    ));
                }
            };
            let Some(base) = view.base() else {
                node.regions.push(ManagedSqliteShmRegionMapping {
                    mapping,
                    view: Some(view),
                    logical_pointer: None,
                    logical_length,
                    mapped_length,
                    aligned_offset,
                });
                self.mark_poisoned(
                    &mut state,
                    ManagedSqliteShmFailurePhase::ViewMap,
                    true,
                    false,
                );
                return Err(ManagedSqliteShmFailure::poisoned_code(
                    ManagedSqliteShmFailurePhase::ViewMap,
                    "NODE_MANAGED_SQLITE_SHM_VIEW_RETURNED_NULL",
                    true,
                    false,
                ));
            };
            // SAFETY: `mapped_length` was checked as shift + logical length, and the view owns
            // exactly that many bytes beginning at `base`.
            let logical_pointer = unsafe { NonNull::new_unchecked(base.as_ptr().add(shift)) };
            node.regions.push(ManagedSqliteShmRegionMapping {
                mapping,
                view: Some(view),
                logical_pointer: Some(logical_pointer),
                logical_length,
                mapped_length,
                aligned_offset,
            });
            node.mapped_bytes = mapped_total;
        }

        let selected = node.regions.get(region as usize).and_then(|selected| {
            selected
                .logical_pointer
                .map(|pointer| (pointer, selected.logical_length))
        });
        let Some((pointer, logical_length)) = selected else {
            self.mark_poisoned(
                &mut state,
                ManagedSqliteShmFailurePhase::ViewMap,
                true,
                false,
            );
            return Err(ManagedSqliteShmFailure::poisoned_code(
                ManagedSqliteShmFailurePhase::ViewMap,
                "NODE_MANAGED_SQLITE_SHM_REGION_CUSTODY_MISSING",
                true,
                false,
            ));
        };
        Ok(ManagedSqliteShmMapOutcome::Mapped(
            ManagedSqliteShmRegionPointer::new(pointer, logical_length, region, self.generation),
        ))
    }
}

impl PinnedManagedSqliteShmConnection {
    pub(crate) fn map(
        &mut self,
        region: u32,
        region_size: NonZeroU32,
        mode: ManagedSqliteShmMapMode,
    ) -> Result<ManagedSqliteShmMapOutcome, ManagedSqliteShmFailure> {
        if !self.active {
            return Err(protocol("NODE_MANAGED_SQLITE_SHM_CONNECTION_INACTIVE"));
        }
        self.coordinator
            .map_connection(self.connection_id, region, region_size, mode)
    }
}

fn request_failure(error: io::Error) -> ManagedSqliteShmFailure {
    ManagedSqliteShmFailure::new(
        ManagedSqliteShmFailurePhase::RequestValidation,
        ManagedSqliteShmFailureClass::ProtocolViolation,
        error,
    )
}

fn protocol(code: &'static str) -> ManagedSqliteShmFailure {
    ManagedSqliteShmFailure::code(
        ManagedSqliteShmFailurePhase::RequestValidation,
        ManagedSqliteShmFailureClass::ProtocolViolation,
        code,
    )
}

fn classify_platform(error: &io::Error) -> ManagedSqliteShmFailureClass {
    if error.kind() == io::ErrorKind::Unsupported {
        ManagedSqliteShmFailureClass::PlatformUnsupported
    } else {
        ManagedSqliteShmFailureClass::IoBeforeMutation
    }
}

fn mutation_class(already_mutated: bool, error: &io::Error) -> ManagedSqliteShmFailureClass {
    if already_mutated {
        ManagedSqliteShmFailureClass::MutatedButKnown
    } else if error.kind() == io::ErrorKind::Unsupported {
        ManagedSqliteShmFailureClass::PlatformUnsupported
    } else {
        ManagedSqliteShmFailureClass::IoBeforeMutation
    }
}
