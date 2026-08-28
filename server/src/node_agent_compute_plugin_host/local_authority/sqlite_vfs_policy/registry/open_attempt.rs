//! Ownership seam with no safe producer for one future production handle-bound SQLite open attempt.
//!
//! The process seal has no constructor in this source slice. Even if a future owner mints it,
//! this module can only move an exact authority-open intent through registry PendingMain and
//! Opening. It cannot produce a SQLite connection or an opened local authority.

use std::{
    ffi::{CStr, CString},
    fmt,
    marker::PhantomData,
    rc::Rc,
};

use super::{
    owner::ManagedSqliteRegistryRouteHandle,
    process_owner::{
        ComputePluginHandleBoundSqliteProcessOwner,
        ManagedSqliteRegistryProcessRegistrationRejection,
        ManagedSqliteRegistryProcessRouteRejection,
    },
    types::ManagedSqliteRegistryTerminalReason,
};
use crate::node_agent_compute_plugin_host::local_authority::{
    ComputePluginAuthorityInstanceBinding, ComputePluginHandleBoundAuthorityOpenIntent,
};

/// Borrow of the one process-lifetime registry owner that a future production VFS must own.
///
/// Private fields and the deliberate absence of a constructor keep every source path here
/// unreachable until registration ownership and VFS lifecycle are accepted together.
#[must_use = "the sealed process gate must register an exact authority-open intent"]
pub(super) struct ComputePluginHandleBoundOpenAttemptProcess {
    owner: &'static ComputePluginHandleBoundSqliteProcessOwner,
    _not_send_or_sync: PhantomData<Rc<()>>,
}

impl ComputePluginHandleBoundOpenAttemptProcess {
    pub(super) fn register(
        &self,
        intent: ComputePluginHandleBoundAuthorityOpenIntent,
    ) -> Result<
        RegisteredComputePluginHandleBoundAuthorityOpenAttempt,
        ComputePluginHandleBoundOpenAttemptRegistrationFailure,
    > {
        let identity = ComputePluginHandleBoundOpenIdentity::from_intent(&intent);
        let route = match self.owner.register(intent) {
            Ok(route) => route,
            Err(failure) => {
                let (reason, intent) = failure.into_parts();
                return Err(ComputePluginHandleBoundOpenAttemptRegistrationFailure {
                    reason,
                    intent,
                });
            }
        };
        Ok(RegisteredComputePluginHandleBoundAuthorityOpenAttempt {
            owner: self.owner,
            route: Some(route),
            identity: Some(identity),
            _not_send_or_sync: PhantomData,
        })
    }
}

impl fmt::Debug for ComputePluginHandleBoundOpenAttemptProcess {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginHandleBoundOpenAttemptProcess")
            .field("owner", &"<sealed-process-lifetime-owner>")
            .field("producer", &"absent")
            .finish()
    }
}

#[must_use = "registration failure retains the complete authority-open intent"]
pub(super) struct ComputePluginHandleBoundOpenAttemptRegistrationFailure {
    reason: ManagedSqliteRegistryProcessRegistrationRejection,
    intent: ComputePluginHandleBoundAuthorityOpenIntent,
}

impl ComputePluginHandleBoundOpenAttemptRegistrationFailure {
    pub(super) fn into_parts(
        self,
    ) -> (
        ManagedSqliteRegistryProcessRegistrationRejection,
        ComputePluginHandleBoundAuthorityOpenIntent,
    ) {
        (self.reason, self.intent)
    }
}

impl fmt::Debug for ComputePluginHandleBoundOpenAttemptRegistrationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginHandleBoundOpenAttemptRegistrationFailure")
            .field("reason", &self.reason)
            .field("intent", &"<retained-complete-custody>")
            .finish()
    }
}

/// Exact registry PendingMain custody plus a descriptor copied only from the consumed intent.
#[must_use = "registered pending custody must begin once or retire its exact route"]
pub(super) struct RegisteredComputePluginHandleBoundAuthorityOpenAttempt {
    owner: &'static ComputePluginHandleBoundSqliteProcessOwner,
    route: Option<ManagedSqliteRegistryRouteHandle>,
    identity: Option<ComputePluginHandleBoundOpenIdentity>,
    _not_send_or_sync: PhantomData<Rc<()>>,
}

impl RegisteredComputePluginHandleBoundAuthorityOpenAttempt {
    pub(super) fn authority_instance_binding(&self) -> &ComputePluginAuthorityInstanceBinding {
        self.identity().authority_instance_binding()
    }

    pub(super) fn begin_open(
        mut self,
    ) -> Result<
        OpeningComputePluginHandleBoundAuthorityOpenAttempt,
        ComputePluginHandleBoundOpenAttemptBeginFailure,
    > {
        let route = self.exact_route();
        let main_logical_name = self
            .owner
            .main_logical_name_owned(route)
            .map_err(|reason| ComputePluginHandleBoundOpenAttemptBeginFailure {
                stage: ComputePluginHandleBoundOpenAttemptBeginStage::MainLogicalName,
                reason,
            })?;
        let identity = self
            .identity
            .take()
            .expect("live registered attempt must retain its intent-derived identity");
        let route = self
            .route
            .take()
            .expect("exact-route validation must leave the registered route present");
        match self.owner.begin_open_attempt(route) {
            Ok(()) => Ok(OpeningComputePluginHandleBoundAuthorityOpenAttempt {
                owner: self.owner,
                route: Some(route),
                identity,
                main_logical_name,
                _not_send_or_sync: PhantomData,
            }),
            Err(reason) => {
                self.route = Some(route);
                self.identity = Some(identity);
                Err(ComputePluginHandleBoundOpenAttemptBeginFailure {
                    stage: ComputePluginHandleBoundOpenAttemptBeginStage::RegistryOpening,
                    reason,
                })
            }
        }
    }

    fn exact_route(&self) -> ManagedSqliteRegistryRouteHandle {
        self.route
            .expect("live registered attempt must retain its exact route")
    }

    fn identity(&self) -> &ComputePluginHandleBoundOpenIdentity {
        self.identity
            .as_ref()
            .expect("live registered attempt must retain its intent-derived identity")
    }
}

impl Drop for RegisteredComputePluginHandleBoundAuthorityOpenAttempt {
    fn drop(&mut self) {
        if let Some(route) = self.route.take() {
            let _ = self.owner.retire_pending(route);
        }
    }
}

impl fmt::Debug for RegisteredComputePluginHandleBoundAuthorityOpenAttempt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RegisteredComputePluginHandleBoundAuthorityOpenAttempt")
            .field("phase", &"registered_pending")
            .field("route", &"<opaque-exact-route>")
            .field("identity", &self.identity)
            .finish()
    }
}

#[derive(Debug)]
enum ComputePluginHandleBoundOpenAttemptBeginStage {
    MainLogicalName,
    RegistryOpening,
}

#[must_use = "begin failure has consumed the pending retry capability"]
pub(super) struct ComputePluginHandleBoundOpenAttemptBeginFailure {
    stage: ComputePluginHandleBoundOpenAttemptBeginStage,
    reason: ManagedSqliteRegistryProcessRouteRejection,
}

impl fmt::Debug for ComputePluginHandleBoundOpenAttemptBeginFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginHandleBoundOpenAttemptBeginFailure")
            .field("stage", &self.stage)
            .field("reason", &self.reason)
            .field("retry_capability", &"absent")
            .finish()
    }
}

/// Registry Opening custody before any SQLite connection acceptance exists.
///
/// There is intentionally no success consumer. Abandonment quarantines the route and permanently
/// retains its intent instead of pretending that a PendingMain retirement or connection close ran.
#[must_use = "pre-connection opening custody must be consumed by a future verified VFS handoff"]
pub(super) struct OpeningComputePluginHandleBoundAuthorityOpenAttempt {
    owner: &'static ComputePluginHandleBoundSqliteProcessOwner,
    route: Option<ManagedSqliteRegistryRouteHandle>,
    identity: ComputePluginHandleBoundOpenIdentity,
    main_logical_name: CString,
    _not_send_or_sync: PhantomData<Rc<()>>,
}

impl OpeningComputePluginHandleBoundAuthorityOpenAttempt {
    pub(super) fn authority_instance_binding(&self) -> &ComputePluginAuthorityInstanceBinding {
        self.identity.authority_instance_binding()
    }

    pub(super) fn main_logical_name(&self) -> &CStr {
        &self.main_logical_name
    }
}

impl Drop for OpeningComputePluginHandleBoundAuthorityOpenAttempt {
    fn drop(&mut self) {
        if let Some(route) = self.route.take() {
            let _ = self.owner.retain_terminal_custody(
                route,
                ManagedSqliteRegistryTerminalReason::FailureCustodyRetained,
                ComputePluginHandleBoundOpenAttemptAbandonment,
            );
        }
    }
}

impl fmt::Debug for OpeningComputePluginHandleBoundAuthorityOpenAttempt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpeningComputePluginHandleBoundAuthorityOpenAttempt")
            .field("phase", &"opening_pre_connection")
            .field("route", &"<opaque-exact-route>")
            .field("identity", &self.identity)
            .field("main_logical_name", &"<opaque-one-shot-name>")
            .field("success_consumer", &"absent")
            .finish()
    }
}

struct ComputePluginHandleBoundOpenAttemptAbandonment;

struct ComputePluginHandleBoundOpenIdentity {
    installation_id_digest: String,
    root_identity_digest: String,
    authority_file_name: String,
    authority_instance_binding: ComputePluginAuthorityInstanceBinding,
}

impl ComputePluginHandleBoundOpenIdentity {
    fn from_intent(intent: &ComputePluginHandleBoundAuthorityOpenIntent) -> Self {
        Self {
            installation_id_digest: intent.installation_id_digest().to_owned(),
            root_identity_digest: intent.root_identity_digest().to_owned(),
            authority_file_name: intent.authority_file_name().to_owned(),
            authority_instance_binding: intent.authority_instance_binding().clone(),
        }
    }

    fn authority_instance_binding(&self) -> &ComputePluginAuthorityInstanceBinding {
        &self.authority_instance_binding
    }
}

impl fmt::Debug for ComputePluginHandleBoundOpenIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginHandleBoundOpenIdentity")
            .field("installation_id_digest", &"<redacted>")
            .field("root_identity_digest", &"<redacted>")
            .field("authority_file_name", &self.authority_file_name)
            .field("authority_instance", &"<process-local>")
            .finish()
    }
}
