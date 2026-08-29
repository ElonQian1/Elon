//! xShmUnmap custody plus its exact active-SHM callback-completion boundary.

use super::*;

impl<Custody, NonceSource> ManagedSqliteRegistryPinnedFile<Custody, NonceSource>
where
    Custody: ManagedSqliteRegistryCustody + 'static,
    NonceSource: ManagedSqliteRegistryNonceSource + 'static,
{
    pub(in super::super) fn shm_unmap(
        &mut self,
        mode: ManagedSqliteShmUnmapMode,
    ) -> Result<(), ManagedSqliteRegistryPinnedFileOperationRejection> {
        #[cfg(test)]
        let observe_unmap = self.unmap_runtime_observation_enabled()?;
        #[cfg(test)]
        if observe_unmap {
            self.observe_unmap_runtime_event(
                ManagedSqliteRegistryUnmapRuntimeEvent::CallbackBeginAttempt,
            )?;
        }
        let callback = self
            .owner
            .begin_callback(self.route, ManagedSqliteRegistryCallbackKind::Shm)
            .map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Registry)?;
        #[cfg(test)]
        if observe_unmap {
            self.observe_unmap_runtime_event(
                ManagedSqliteRegistryUnmapRuntimeEvent::CallbackBeginSuccess,
            )?;
        }
        #[cfg(test)]
        let was_attached = self.unmap_shm_is_attached();
        let result = self.unmap_shm_custody(mode);
        #[cfg(test)]
        if observe_unmap && was_attached && !self.unmap_shm_is_attached() {
            // SharedNonFinal counts a selected detach attempt only once the real coordinator has
            // crossed every validation/pre-call gate and its attachment actually transitioned.
            // The adjacent pair is an append-only witness of that native boundary; held-lock and
            // detach-before rejections therefore record neither event.
            self.observe_unmap_runtime_event(
                ManagedSqliteRegistryUnmapRuntimeEvent::SelectedActionAttempt,
            )?;
            self.observe_unmap_runtime_event(
                ManagedSqliteRegistryUnmapRuntimeEvent::SelectedActionSuccess,
            )?;
        }
        if let Err(ManagedSqliteRegistryPinnedFileOperationRejection::Shm(failure)) = &result {
            self.quarantine_unsafe_shm_failure(failure);
        }

        #[cfg(test)]
        let completion = if observe_unmap {
            (|| {
                let phase =
                    super::super::ManagedSqliteRegistryCloseLifecyclePhase::UnmapCallbackCompletion;
                if self
                    .close_faults
                    .as_ref()
                    .is_some_and(|faults| faults.before(phase).unwrap_or(true))
                {
                    drop(callback);
                    return Err(
                        ManagedSqliteRegistryPinnedFileOperationRejection::InjectedLifecycle,
                    );
                }
                self.observe_unmap_runtime_event(
                    ManagedSqliteRegistryUnmapRuntimeEvent::CallbackCompletionAttempt,
                )?;
                let mut callback = callback;
                let native_rejection_armed = match self.close_faults.as_ref() {
                    Some(faults) => faults.claim_native_failure_gate(phase).map_err(|()| {
                        ManagedSqliteRegistryPinnedFileOperationRejection::InjectedLifecycle
                    })?,
                    None => false,
                };
                if native_rejection_armed {
                    callback
                        .arm_shm_callback_completion_native_rejection()
                        .map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Registry)?;
                }
                let completed = match callback.complete_with_receipt() {
                    Ok(completed) => {
                        if self
                            .observe_unmap_runtime_event(
                                ManagedSqliteRegistryUnmapRuntimeEvent::CallbackCompletionSuccess,
                            )
                            .is_err()
                        {
                            let _ = self.owner.retain_terminal_custody(
                                self.route,
                                ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                                completed,
                            );
                            return Err(
                                ManagedSqliteRegistryPinnedFileOperationRejection::InjectedLifecycle,
                            );
                        }
                        completed
                    }
                    Err(rejection) => {
                        if native_rejection_armed {
                            if let Some(faults) = self.close_faults.as_ref() {
                                faults.native_failure(phase);
                            }
                        }
                        return Err(ManagedSqliteRegistryPinnedFileOperationRejection::Registry(
                            rejection,
                        ));
                    }
                };
                if self
                    .close_faults
                    .as_ref()
                    .is_some_and(|faults| faults.after_success(phase).unwrap_or(true))
                {
                    let _ = self.owner.retain_terminal_custody(
                        self.route,
                        ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                        completed,
                    );
                    return Err(
                        ManagedSqliteRegistryPinnedFileOperationRejection::InjectedLifecycle,
                    );
                }
                Ok(())
            })()
        } else {
            callback
                .complete_with_receipt()
                .map(drop)
                .map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Registry)
        };
        #[cfg(not(test))]
        let completion = callback
            .complete()
            .map_err(ManagedSqliteRegistryPinnedFileOperationRejection::Registry);

        match (result, completion) {
            (Err(rejection), _) => Err(rejection),
            (Ok(()), Err(rejection)) => Err(rejection),
            (Ok(()), Ok(())) => Ok(()),
        }
    }

    fn unmap_shm_custody(
        &mut self,
        mode: ManagedSqliteShmUnmapMode,
    ) -> Result<(), ManagedSqliteRegistryPinnedFileOperationRejection> {
        let custody = self
            .custody
            .take()
            .expect("live SHM unmap must retain exact custody");
        let ManagedSqliteRegistryPinnedFileCustody::WalMain {
            mut file,
            main,
            shm,
        } = custody
        else {
            self.custody = Some(custody);
            return Err(ManagedSqliteRegistryPinnedFileOperationRejection::UnsupportedFileRole);
        };
        if file.shm_mut().is_none() {
            self.custody =
                Some(ManagedSqliteRegistryPinnedFileCustody::WalMain { file, main, shm });
            return Err(ManagedSqliteRegistryPinnedFileOperationRejection::ShmDetached);
        }
        match file.unmap_shm(mode) {
            Ok(file) => {
                self.custody =
                    Some(ManagedSqliteRegistryPinnedFileCustody::WalMain { file, main, shm });
                Ok(())
            }
            Err(failure) => {
                let (failure, file) = failure.into_parts();
                self.custody =
                    Some(ManagedSqliteRegistryPinnedFileCustody::WalMain { file, main, shm });
                Err(ManagedSqliteRegistryPinnedFileOperationRejection::Shm(
                    failure,
                ))
            }
        }
    }

    #[cfg(test)]
    fn observe_unmap_runtime_event(
        &self,
        event: ManagedSqliteRegistryUnmapRuntimeEvent,
    ) -> Result<(), ManagedSqliteRegistryPinnedFileOperationRejection> {
        match self.close_faults.as_ref() {
            Some(faults) => faults
                .observe_unmap_runtime_event(event)
                .map_err(|()| ManagedSqliteRegistryPinnedFileOperationRejection::InjectedLifecycle),
            None => Ok(()),
        }
    }

    #[cfg(test)]
    fn unmap_runtime_observation_enabled(
        &self,
    ) -> Result<bool, ManagedSqliteRegistryPinnedFileOperationRejection> {
        match self.close_faults.as_ref() {
            Some(faults) => faults
                .unmap_runtime_observation_enabled()
                .map_err(|()| ManagedSqliteRegistryPinnedFileOperationRejection::InjectedLifecycle),
            None => Ok(false),
        }
    }

    #[cfg(test)]
    fn unmap_shm_is_attached(&self) -> bool {
        match self.custody.as_ref() {
            Some(ManagedSqliteRegistryPinnedFileCustody::WalMain { file, .. }) => {
                file.unmap_shm_connection_active_for_test()
            }
            _ => false,
        }
    }
}
