//! Process-isolated real xClose harness for the frozen 36-case JointClose family.

use std::path::Path;

use super::a2b2_cases::{JointCloseActual, JointCloseSelector};

mod action;
mod boundary;
mod counts;
mod custody;
mod invoke;
mod observe;
mod outcome;
mod prepare;
mod retry;
mod runtime;
mod shm;
mod stimulus;

pub(super) fn exercise_joint_close(
    root: &Path,
    selector: JointCloseSelector,
) -> anyhow::Result<JointCloseActual> {
    let mut fixture = prepare::JointCloseFixture::prepare(root, selector)?;
    stimulus::install(&fixture, selector)?;
    let first = invoke::first(&mut fixture, selector)?;
    let actual = observe::seal_after_first(&fixture, selector, first)?;
    retry::invoke_and_validate(&mut fixture, selector, first)?;
    Ok(actual)
}
