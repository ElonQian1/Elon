//! Test-only two-Connection owner for one VFS registration and managed namespace.

use std::{path::Path, sync::Arc};

use anyhow::anyhow;
use rusqlite::Connection;

use super::*;

const CONNECTION_COUNT: usize = 2;

#[cfg(all(test, windows))]
mod registry_lifecycle;

pub(super) struct ManagedSqliteMultiConnectionFixture {
    registration: Option<ManagedTestVfsRegistration>,
    connections: [Option<ManagedSqliteRoutedConnectionFixture>; CONNECTION_COUNT],
}

impl ManagedSqliteMultiConnectionFixture {
    pub(super) fn open(root: &Path, nonce_seed: [u8; 16]) -> anyhow::Result<Self> {
        Self::open_with_count(root, nonce_seed, 2)
    }

    #[cfg(all(test, windows))]
    pub(super) fn open_single(root: &Path, nonce_seed: [u8; 16]) -> anyhow::Result<Self> {
        Self::open_with_count(root, nonce_seed, 1)
    }

    fn open_with_count(
        root: &Path,
        nonce_seed: [u8; 16],
        connection_count: usize,
    ) -> anyhow::Result<Self> {
        let registration = ManagedTestVfsRegistration::register(root, nonce_seed)?;
        let first = ManagedSqliteRoutedConnectionFixture::open_registered(&registration)?;
        let second = if connection_count == 2 {
            Some(ManagedSqliteRoutedConnectionFixture::open_registered(
                &registration,
            )?)
        } else {
            None
        };
        Ok(Self {
            registration: Some(registration),
            connections: [Some(first), second],
        })
    }

    pub(super) fn connection(&self, index: usize) -> anyhow::Result<&Connection> {
        self.connections
            .get(index)
            .and_then(Option::as_ref)
            .map(ManagedSqliteRoutedConnectionFixture::connection)
            .ok_or_else(|| anyhow!("managed SQLite connection {index} is not live"))
    }

    pub(super) fn route(
        &self,
        index: usize,
    ) -> anyhow::Result<&ManagedSqliteRoutedConnectionFixture> {
        self.connections
            .get(index)
            .and_then(Option::as_ref)
            .ok_or_else(|| anyhow!("managed SQLite route {index} is not live"))
    }

    pub(super) fn route_ordinal(&self, index: usize) -> anyhow::Result<ManagedTestRouteOrdinal> {
        Ok(self.route(index)?.route_ordinal())
    }

    #[cfg(all(test, windows))]
    pub(super) fn live_connection_count(&self) -> usize {
        self.connections
            .iter()
            .filter(|slot| slot.is_some())
            .count()
    }

    #[cfg(all(test, windows))]
    pub(super) fn logical_route_counts(&self) -> anyhow::Result<(usize, usize)> {
        let snapshot = self
            .registration
            .as_ref()
            .expect("managed VFS registration")
            .routes()
            .registration_shutdown_snapshot()?;
        Ok((snapshot.live_routes(), snapshot.logical_names()))
    }

    #[cfg(all(test, windows))]
    pub(super) fn live_registration_snapshot(
        &self,
    ) -> anyhow::Result<ManagedTestVfsLiveRegistrationSnapshot> {
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .live_registration_snapshot()
    }

    pub(super) fn install_callback_fault_script(
        &self,
        steps: &[ManagedTestCallbackFaultStep],
    ) -> Result<(), &'static str> {
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .faults()
            .install(steps)
    }

    pub(super) fn callback_fault_controller(&self) -> Arc<ManagedTestCallbackFaultController> {
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .faults()
    }

    pub(super) fn install_lifecycle_fault_script(
        &self,
        steps: &[ManagedTestLifecycleFaultStep],
    ) -> Result<(), &'static str> {
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .lifecycle()
            .install(steps)
    }

    #[cfg(all(test, windows))]
    pub(super) fn begin_unfaulted_barrier_observation_window(
        &self,
        route: ManagedTestRouteOrdinal,
    ) -> Result<(), &'static str> {
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .lifecycle()
            .begin_unfaulted_barrier_observation_window(route)
    }

    pub(super) fn pending_lifecycle_fault_count(&self) -> Result<usize, &'static str> {
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .lifecycle()
            .pending_count()
    }

    pub(super) fn lifecycle_fault_observations(
        &self,
    ) -> Result<Vec<ManagedTestLifecycleFaultObservation>, &'static str> {
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .lifecycle()
            .observations()
    }

    pub(super) fn pending_callback_fault_count(&self) -> Result<usize, &'static str> {
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .faults()
            .pending_count()
    }

    pub(super) fn callback_fault_observations(
        &self,
    ) -> Result<Vec<ManagedTestCallbackFaultObservation>, &'static str> {
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .faults()
            .observations()
    }

    pub(super) fn close_connection(
        &mut self,
        index: usize,
    ) -> anyhow::Result<ManagedTestVfsCounts> {
        self.connections
            .get_mut(index)
            .and_then(Option::take)
            .ok_or_else(|| anyhow!("managed SQLite connection {index} is not live"))?
            .close()
    }

    pub(super) fn live_route_count(&self) -> anyhow::Result<usize> {
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .live_route_count()
    }

    pub(super) fn counts(&self) -> ManagedTestVfsCounts {
        self.registration
            .as_ref()
            .expect("managed VFS registration")
            .counts()
    }

    pub(super) fn close(mut self) -> anyhow::Result<ManagedTestVfsCounts> {
        for index in 0..CONNECTION_COUNT {
            if let Some(connection) = self.connections[index].take() {
                connection.close()?;
            }
        }
        let registration = self.registration.take().expect("managed VFS registration");
        let counts = registration.counts();
        registration.unregister()?;
        Ok(counts)
    }
}

impl Drop for ManagedSqliteMultiConnectionFixture {
    fn drop(&mut self) {
        for connection in &mut self.connections {
            drop(connection.take());
        }
        drop(self.registration.take());
    }
}
