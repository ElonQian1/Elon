use std::io::ErrorKind;

use super::{
    child::{
        ChildIdentityFingerprint, RegistrationCommitment, RootCommitment,
        ValidatedChildProcessReceipt,
    },
    environment::WindowsDynamicEnvironment,
};

/// Parent-only proof that it removed the same child-bound root after successful child exit.
pub(in super::super) struct ValidatedParentCleanupReceipt {
    pub(super) child_fingerprint: ChildIdentityFingerprint,
    pub(super) root_commitment: RootCommitment,
    pub(super) registration_commitment: RegistrationCommitment,
}

impl ValidatedParentCleanupReceipt {
    /// Performs deletion internally; a caller cannot substitute rename or a pre-absent root.
    pub(in super::super) fn remove_after_child_exit(
        child: &ValidatedChildProcessReceipt,
        environment: &WindowsDynamicEnvironment,
    ) -> Result<Self, &'static str> {
        let child_fingerprint = child.fingerprint();
        if child_fingerprint != environment.child_fingerprint
            || child.root_commitment != environment.root_commitment
            || child.registration_commitment != environment.registration_commitment
        {
            return Err("A2_DYNAMIC_CLEANUP_BINDING_MISMATCH");
        }
        std::fs::remove_dir_all(environment.root_for_cleanup())
            .map_err(|_| "A2_DYNAMIC_PARENT_CLEANUP_REMOVE_FAILED")?;
        match std::fs::symlink_metadata(environment.root_for_cleanup()) {
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Self {
                child_fingerprint,
                root_commitment: RootCommitment(environment.root_commitment.0),
                registration_commitment: RegistrationCommitment(
                    environment.registration_commitment.0,
                ),
            }),
            Err(_) => Err("A2_DYNAMIC_PARENT_CLEANUP_NOT_OBSERVABLE"),
            Ok(_) => Err("A2_DYNAMIC_PARENT_CLEANUP_ROOT_REMAINS"),
        }
    }
}
