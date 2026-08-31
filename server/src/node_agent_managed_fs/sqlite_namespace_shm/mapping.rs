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

#[cfg(all(test, windows))]
use super::test_map_runtime::{
    ManagedSqliteShmTestMapDmsPath, ManagedSqliteShmTestMapPath, ManagedSqliteShmTestMapStep,
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

        #[cfg(all(test, windows))]
        let test_request = (region, region_size.get(), mode);
        #[cfg(all(test, windows))]
        self.begin_test_map_action(connection_id, region, region_size.get(), mode)?;
        #[cfg(all(test, windows))]
        self.record_test_map_dms_path(
            connection_id,
            test_request,
            if state.node.is_some() {
                ManagedSqliteShmTestMapDmsPath::NodeLive
            } else {
                ManagedSqliteShmTestMapDmsPath::CreatedFirstShared
            },
        )?;

        let initialization_mutated = {
            let (node, initialization_mutated) = self.ensure_node(&mut state, connection_id)?;
            match node.region_size {
                Some(expected) if expected != region_size => {
                    return Err(protocol("NODE_MANAGED_SQLITE_SHM_REGION_SIZE_CHANGED"));
                }
                None => node.region_size = Some(region_size),
                Some(_) => {}
            }
            initialization_mutated
        };
        #[cfg(all(test, windows))]
        self.record_test_map_dms_ready(connection_id, test_request, initialization_mutated)?;
        #[cfg(test)]
        let file_size_fault = self.begin_test_fault(
            &mut state,
            connection_id,
            ManagedSqliteShmFailurePhase::FileSize,
            initialization_mutated,
        )?;
        let current_size = state
            .node
            .as_mut()
            .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_BEFORE_SIZE"))?
            .file
            .size()
            .map_err(|error| {
                let error = io::Error::other(error);
                ManagedSqliteShmFailure::new(
                    ManagedSqliteShmFailurePhase::FileSize,
                    mutation_class(initialization_mutated, &error),
                    error,
                )
            })?;
        #[cfg(test)]
        if let Some(failure) =
            self.finish_test_fault(&mut state, file_size_fault, initialization_mutated)
        {
            return Err(failure);
        }
        self.budget
            .validate_existing_size(current_size)
            .map_err(request_failure)?;
        #[cfg(all(test, windows))]
        self.record_test_map_file_size(
            connection_id,
            test_request,
            current_size,
            logical_end,
            initialization_mutated,
        )?;
        let mut file_grew = false;
        if current_size < logical_end {
            if mode == ManagedSqliteShmMapMode::Observe {
                #[cfg(all(test, windows))]
                {
                    self.record_test_map_step(
                        connection_id,
                        test_request,
                        ManagedSqliteShmFailurePhase::FileSize,
                        initialization_mutated,
                        ManagedSqliteShmTestMapStep::NotPresent,
                    )?;
                    self.finish_test_map_action(
                        connection_id,
                        test_request,
                        initialization_mutated,
                    )?;
                }
                return Ok(ManagedSqliteShmMapOutcome::NotPresent);
            }
            #[cfg(test)]
            let file_grow_fault = self.begin_test_fault(
                &mut state,
                connection_id,
                ManagedSqliteShmFailurePhase::FileGrow,
                initialization_mutated,
            )?;
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
            #[cfg(test)]
            if let Some(failure) = self.finish_test_fault(&mut state, file_grow_fault, true) {
                return Err(failure);
            }
            #[cfg(all(test, windows))]
            self.record_test_map_step(
                connection_id,
                test_request,
                ManagedSqliteShmFailurePhase::FileGrow,
                true,
                ManagedSqliteShmTestMapStep::FileGrow,
            )?;
        }

        #[cfg(all(test, windows))]
        let mut mapped_new = false;
        #[cfg(all(test, windows))]
        let map_loop_start = state
            .node
            .as_ref()
            .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_BEFORE_MAP"))?
            .regions
            .len();
        while state
            .node
            .as_ref()
            .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_BEFORE_MAP"))?
            .regions
            .len()
            <= region as usize
        {
            let existing_regions = state
                .node
                .as_ref()
                .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_DURING_MAP"))?
                .regions
                .len();
            let index = u32::try_from(existing_regions).map_err(|_| {
                request_failure(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "NODE_MANAGED_SQLITE_SHM_REGION_INDEX_OVERFLOW",
                ))
            })?;
            #[cfg(all(test, windows))]
            let loop_ordinal = u16::try_from(existing_regions - map_loop_start).map_err(|_| {
                request_failure(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "NODE_MANAGED_SQLITE_SHM_TEST_MAP_LOOP_ORDINAL_OVERFLOW",
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
            let mapped_total = state
                .node
                .as_ref()
                .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_BEFORE_BUDGET"))?
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

            let prior_mapping_mutation =
                initialization_mutated || file_grew || existing_regions != 0;
            #[cfg(test)]
            let mapping_fault = self.begin_test_fault(
                &mut state,
                connection_id,
                ManagedSqliteShmFailurePhase::MappingCreate,
                prior_mapping_mutation,
            )?;
            let mut mapping = {
                let node = state.node.as_ref().ok_or_else(|| {
                    protocol("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_BEFORE_MAPPING_CREATE")
                })?;
                platform_shm::create_mapping(&node.file.file, logical_end)
            }
            .map_err(|error| {
                ManagedSqliteShmFailure::new(
                    ManagedSqliteShmFailurePhase::MappingCreate,
                    mutation_class(prior_mapping_mutation, &error),
                    error,
                )
            })?;
            #[cfg(test)]
            if mapping_fault.is_some() {
                self.retain_test_mapping_custody(
                    &mut state,
                    mapping,
                    None,
                    None,
                    logical_length,
                    mapped_length,
                    aligned_offset,
                )?;
                if let Some(failure) = self.finish_test_fault(&mut state, mapping_fault, true) {
                    return Err(failure);
                }
                self.mark_poisoned(
                    &mut state,
                    ManagedSqliteShmFailurePhase::MappingCreate,
                    true,
                    false,
                );
                return Err(ManagedSqliteShmFailure::poisoned_code(
                    ManagedSqliteShmFailurePhase::MappingCreate,
                    "NODE_MANAGED_SQLITE_SHM_TEST_FAULT_AFTER_MATCH_LOST",
                    true,
                    false,
                ));
            }
            #[cfg(all(test, windows))]
            self.record_test_map_step(
                connection_id,
                test_request,
                ManagedSqliteShmFailurePhase::MappingCreate,
                true,
                ManagedSqliteShmTestMapStep::MappingCreate(loop_ordinal),
            )?;
            #[cfg(test)]
            let view_fault = match self.begin_test_fault(
                &mut state,
                connection_id,
                ManagedSqliteShmFailurePhase::ViewMap,
                true,
            ) {
                Ok(fault) => fault,
                Err(failure)
                    if failure.class()
                        == ManagedSqliteShmFailureClass::OutcomeUncertainPoisoned =>
                {
                    self.retain_test_mapping_custody(
                        &mut state,
                        mapping,
                        None,
                        None,
                        logical_length,
                        mapped_length,
                        aligned_offset,
                    )?;
                    return Err(failure);
                }
                Err(failure) => {
                    if let Err(close_error) = mapping.close_explicit() {
                        self.retain_test_mapping_custody(
                            &mut state,
                            mapping,
                            None,
                            None,
                            logical_length,
                            mapped_length,
                            aligned_offset,
                        )?;
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
                    return Err(failure);
                }
            };
            let view = match platform_shm::map_view(&mapping, aligned_offset, mapped_length) {
                Ok(view) => view,
                Err(error) => {
                    if let Err(close_error) = mapping.close_explicit() {
                        state
                            .node
                            .as_mut()
                            .ok_or_else(|| {
                                protocol("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_AT_MAP_FAILURE")
                            })?
                            .regions
                            .push(ManagedSqliteShmRegionMapping {
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
                        mutation_class(prior_mapping_mutation, &error),
                        error,
                    ));
                }
            };
            let Some(base) = view.base() else {
                state
                    .node
                    .as_mut()
                    .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_AT_NULL_VIEW"))?
                    .regions
                    .push(ManagedSqliteShmRegionMapping {
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
            #[cfg(all(test, windows))]
            self.record_test_map_step(
                connection_id,
                test_request,
                ManagedSqliteShmFailurePhase::ViewMap,
                true,
                ManagedSqliteShmTestMapStep::ViewMap(loop_ordinal),
            )?;
            let node = state
                .node
                .as_mut()
                .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_AFTER_VIEW_MAP"))?;
            node.regions.push(ManagedSqliteShmRegionMapping {
                mapping,
                view: Some(view),
                logical_pointer: Some(logical_pointer),
                logical_length,
                mapped_length,
                aligned_offset,
            });
            node.mapped_bytes = mapped_total;
            #[cfg(all(test, windows))]
            {
                self.record_test_map_step(
                    connection_id,
                    test_request,
                    ManagedSqliteShmFailurePhase::ViewMap,
                    true,
                    ManagedSqliteShmTestMapStep::Record(loop_ordinal),
                )?;
                mapped_new = true;
            }
            #[cfg(test)]
            if let Some(failure) = self.finish_test_fault(&mut state, view_fault, true) {
                return Err(failure);
            }
        }

        let node = state
            .node
            .as_mut()
            .ok_or_else(|| protocol("NODE_MANAGED_SQLITE_SHM_NODE_MISSING_AFTER_MAP"))?;
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
        #[cfg(all(test, windows))]
        {
            self.record_test_map_selected(
                connection_id,
                test_request,
                if mapped_new {
                    ManagedSqliteShmTestMapPath::MappedNew
                } else {
                    ManagedSqliteShmTestMapPath::MappedReuse
                },
                pointer,
                logical_length,
            )?;
            self.finish_test_map_action(
                connection_id,
                test_request,
                initialization_mutated || file_grew || mapped_new,
            )?;
        }
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
