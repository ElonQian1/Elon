//! Exact outer Close-callback runtime trace for one real xClose invocation.

use anyhow::anyhow;

use super::{super::a2b2_cases::JointCloseSelector as S, outcome};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_policy::registry::{
    ManagedSqliteRegistryLifecycleStage as Stage, ManagedSqliteRegistryUnmapRuntimeEvent as Event,
};

pub(super) fn validate(selector: S, events: &[Event], stages: &[Stage]) -> anyhow::Result<()> {
    use Event as E;

    let expected: &[Event] = match selector {
        S::RawStateTakeRejected | S::BeginConnectionCloseRejected | S::CallbackWrapperBefore => &[],
        S::CallbackAdmissionRejected => &[E::CallbackBeginAttempt],
        selector if outcome::is_shm(selector) || outcome::is_main(selector) => &[
            E::CallbackBeginAttempt,
            E::CallbackBeginSuccess,
            E::CallbackCompletionAttempt,
        ],
        S::PhysicalSuccess => &[E::CallbackBeginAttempt, E::CallbackBeginSuccess],
        S::RegistryWalMainCloseBefore | S::RegistryWalMainCloseAfterKnown => &[
            E::CallbackBeginAttempt,
            E::CallbackBeginSuccess,
            E::CallbackCompletionAttempt,
            E::CallbackCompletionSuccess,
        ],
        S::RegistryWalMainCloseNativeUncertain => &[
            E::CallbackBeginAttempt,
            E::CallbackBeginSuccess,
            E::CallbackCompletionAttempt,
        ],
        _ => return Err(anyhow!("JointClose runtime selector is not frozen")),
    };
    if events != expected {
        return Err(anyhow!(
            "JointClose outer runtime trace is not exact for its frozen selector"
        ));
    }

    let count = |expected| -> anyhow::Result<u8> {
        let count = events.iter().filter(|actual| **actual == expected).count();
        if count > 1 {
            return Err(anyhow!(
                "JointClose outer runtime event is not at-most-once"
            ));
        }
        Ok(u8::from(count == 1))
    };
    let callback_begin = count(E::CallbackBeginSuccess)?;
    let callback_complete_attempt = count(E::CallbackCompletionAttempt)?;
    let callback_complete_success = count(E::CallbackCompletionSuccess)?;
    if callback_begin != stage_count(stages, Stage::CallbackBegin)?
        || callback_complete_attempt != stage_count(stages, Stage::CallbackCompletionAttempt)?
        || callback_complete_success != stage_count(stages, Stage::CallbackCompletionSucceeded)?
    {
        return Err(anyhow!(
            "JointClose outer runtime receipt disagrees with the real callback lifecycle"
        ));
    }
    Ok(())
}

fn stage_count(stages: &[Stage], expected: Stage) -> anyhow::Result<u8> {
    let count = stages.iter().filter(|actual| **actual == expected).count();
    if count > 1 {
        return Err(anyhow!("JointClose lifecycle stage is not at-most-once"));
    }
    Ok(u8::from(count == 1))
}
