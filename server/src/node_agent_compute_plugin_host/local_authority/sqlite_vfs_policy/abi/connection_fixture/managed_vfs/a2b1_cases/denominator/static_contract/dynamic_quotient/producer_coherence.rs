//! Closed producer relations for typed terminal descriptors.
//!
//! This layer validates only typed values emitted beside the source graph. It deliberately has no
//! access to leaf ids, branch labels, debug strings, or display text.

mod lock;
mod lock_axes;
mod map;
mod map_axes;

use super::super::terminal_descriptor::{
    CapabilityGapV1, ExecutionRecipeV1, InitializationFaultSiteV1, InitializationPathV1,
    InitializationStimulusV1, PhaseV1, RunnerCapabilityV1, SourceSiteV1, TerminalDescriptorV1,
    TimingV1,
};
use super::projector::{ProjectionErrorV1, ProjectionViolationV1};

pub(super) fn validate(descriptor: &TerminalDescriptorV1) -> Result<(), ProjectionErrorV1> {
    match descriptor {
        TerminalDescriptorV1::Map(value) => map::validate(*value),
        TerminalDescriptorV1::Lock(value) => lock::validate(*value),
    }
}

pub(super) fn invalid(violation: ProjectionViolationV1) -> ProjectionErrorV1 {
    ProjectionErrorV1::Invalid(violation)
}

pub(super) fn valid_map_capability(recipe: ExecutionRecipeV1) -> bool {
    recipe.capability == RunnerCapabilityV1::Missing(CapabilityGapV1::QuotientRunnerNotIntegrated)
}

pub(super) fn valid_lock_capability(recipe: ExecutionRecipeV1) -> bool {
    recipe.capability == RunnerCapabilityV1::Missing(CapabilityGapV1::LockObservationIncomplete)
}

pub(super) fn valid_stored_poison_phase(phase: PhaseV1) -> bool {
    matches!(
        phase,
        PhaseV1::Gate
            | PhaseV1::FileClose
            | PhaseV1::ExactSiblingDelete
            | PhaseV1::ExactSiblingOpen
            | PhaseV1::DmsTruncate
            | PhaseV1::FileGrow
            | PhaseV1::MappingClose
            | PhaseV1::ViewUnmap
            | PhaseV1::LockRelease
            | PhaseV1::ConnectionDetach
            | PhaseV1::DeleteAuthorization
            | PhaseV1::DmsExclusiveRelease
            | PhaseV1::DmsSharedRelease
    )
}

pub(super) fn valid_initialization_tuple(
    source_site: SourceSiteV1,
    stimulus: InitializationStimulusV1,
    phase: PhaseV1,
    timing: TimingV1,
) -> bool {
    use InitializationFaultSiteV1 as Site;
    use InitializationPathV1 as Path;

    match stimulus.fault_site {
        Site::ParentValidationBeforeOpen
        | Site::ParentHandle
        | Site::PlatformOpen
        | Site::OpenCompletionValidation
        | Site::OpenFileValidation
        | Site::ParentValidationAfterOpen => {
            if source_site != SourceSiteV1::InitializationOpen
                || stimulus.path != Path::Opening
                || stimulus.cleanup_rewrite
            {
                return false;
            }
            (phase == PhaseV1::ExactSiblingOpen && timing == TimingV1::AtCall)
                || (matches!(
                    stimulus.fault_site,
                    Site::OpenCompletionValidation
                        | Site::OpenFileValidation
                        | Site::ParentValidationAfterOpen
                ) && phase == PhaseV1::FileClose
                    && timing == TimingV1::Cleanup)
        }
        Site::DmsExclusiveAcquire => {
            source_site == SourceSiteV1::InitializationDms
                && matches!(stimulus.path, Path::Created | Path::Existing)
                && ((!stimulus.cleanup_rewrite
                    && phase == PhaseV1::DmsExclusiveAcquire
                    && timing == TimingV1::AtCall)
                    || (stimulus.cleanup_rewrite
                        && phase == PhaseV1::FileClose
                        && timing == TimingV1::Cleanup))
        }
        Site::DmsTruncate => {
            source_site == SourceSiteV1::InitializationDms
                && matches!(stimulus.path, Path::CreatedFirst | Path::ExistingFirst)
                && !stimulus.cleanup_rewrite
                && phase == PhaseV1::DmsTruncate
                && timing == TimingV1::AtCall
        }
        Site::DmsExclusiveRelease => {
            source_site == SourceSiteV1::InitializationDms
                && matches!(stimulus.path, Path::CreatedFirst | Path::ExistingFirst)
                && !stimulus.cleanup_rewrite
                && phase == PhaseV1::DmsExclusiveRelease
                && matches!(timing, TimingV1::AtCall | TimingV1::Cleanup)
        }
        Site::DmsSharedAcquire => {
            source_site == SourceSiteV1::InitializationDms
                && matches!(
                    stimulus.path,
                    Path::CreatedFirst
                        | Path::ExistingFirst
                        | Path::CreatedJoiner
                        | Path::ExistingJoiner
                )
                && ((!stimulus.cleanup_rewrite
                    && phase == PhaseV1::DmsSharedAcquire
                    && timing == TimingV1::AtCall)
                    || (stimulus.cleanup_rewrite
                        && phase == PhaseV1::FileClose
                        && timing == TimingV1::Cleanup))
        }
    }
}
