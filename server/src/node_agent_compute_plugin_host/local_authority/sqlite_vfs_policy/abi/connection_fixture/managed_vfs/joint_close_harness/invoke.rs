//! Exact first/second calls through the saved real xClose callback.

use anyhow::anyhow;
use rusqlite::ffi;

use super::{super::a2b2_cases::JointCloseSelector, prepare::JointCloseFixture};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi::HandleBoundSqliteAbiRawCloseWitnessSnapshot;

#[derive(Debug, Clone, Copy)]
pub(super) struct FirstClose {
    pub(super) code: i32,
    pub(super) raw: HandleBoundSqliteAbiRawCloseWitnessSnapshot,
}

pub(super) fn first(
    fixture: &mut JointCloseFixture,
    selector: JointCloseSelector,
) -> anyhow::Result<FirstClose> {
    let code = fixture.invoke_first()?;
    if selector == JointCloseSelector::PhysicalSuccess {
        require_close_succeeded(code)?;
    } else {
        require_close_rejected(code, "first")?;
    }
    let raw = fixture.raw_witness.snapshot();
    if selector == JointCloseSelector::RawStateTakeRejected {
        require_slots_installed(fixture)?;
        validate_raw_rejection(raw)?;
    } else {
        require_slots_cleared(fixture)?;
        validate_ordinary_raw_close(raw)?;
    }
    Ok(FirstClose { code, raw })
}

pub(super) fn second(
    fixture: &mut JointCloseFixture,
    selector: JointCloseSelector,
    first: FirstClose,
) -> anyhow::Result<i32> {
    let code = fixture.invoke_second()?;
    require_close_rejected(code, "second")?;
    let raw = fixture.raw_witness.snapshot();
    if selector == JointCloseSelector::RawStateTakeRejected {
        require_slots_installed(fixture)?;
        validate_raw_retry_delta(first.raw, raw)?;
    } else if raw != first.raw {
        return Err(anyhow!(
            "ordinary JointClose retry changed the released raw-close witness"
        ));
    }
    Ok(code)
}

fn require_close_rejected(code: i32, invocation: &'static str) -> anyhow::Result<()> {
    if code != ffi::SQLITE_IOERR_CLOSE {
        return Err(anyhow!(
            "JointClose {invocation} saved xClose returned {code}, expected SQLITE_IOERR_CLOSE"
        ));
    }
    Ok(())
}

fn require_close_succeeded(code: i32) -> anyhow::Result<()> {
    if code != ffi::SQLITE_OK {
        return Err(anyhow!(
            "JointClose first physical-success xClose returned {code}, expected SQLITE_OK"
        ));
    }
    Ok(())
}

fn require_slots_cleared(fixture: &JointCloseFixture) -> anyhow::Result<()> {
    let slots = fixture.observe_raw_slots()?;
    if slots.methods_installed || slots.state_installed {
        return Err(anyhow!("JointClose xClose did not clear both raw slots"));
    }
    Ok(())
}

fn require_slots_installed(fixture: &JointCloseFixture) -> anyhow::Result<()> {
    let slots = fixture.observe_raw_slots()?;
    if !slots.methods_installed || !slots.state_installed {
        return Err(anyhow!(
            "JointClose raw-state rejection did not preserve both installed slots"
        ));
    }
    Ok(())
}

fn validate_ordinary_raw_close(
    raw: HandleBoundSqliteAbiRawCloseWitnessSnapshot,
) -> anyhow::Result<()> {
    if raw.raw_close_entries != 1
        || raw.raw_close_entry_order != 1
        || raw.state_take_attempts != 1
        || raw.state_take_attempt_order != 2
        || raw.methods_clears != 1
        || raw.methods_clear_order != 3
        || raw.state_take_successes != 1
        || raw.state_take_success_order != 4
        || raw.state_close_custody_retentions != 0
        || raw.state_close_custody_retention_order != 0
        || raw.state_close_attempts != 1
        || raw.state_close_attempt_order != 5
        || raw.state_abandons != 0
        || raw.state_abandon_order != 0
    {
        return Err(anyhow!(
            "JointClose ordinary raw xClose receipt is not exact"
        ));
    }
    Ok(())
}

fn validate_raw_rejection(raw: HandleBoundSqliteAbiRawCloseWitnessSnapshot) -> anyhow::Result<()> {
    if raw.raw_close_entries != 1
        || raw.raw_close_entry_order != 1
        || raw.state_take_attempts != 1
        || raw.state_take_attempt_order != 2
        || raw.methods_clears != 0
        || raw.methods_clear_order != 0
        || raw.state_take_successes != 0
        || raw.state_take_success_order != 0
        || raw.state_close_custody_retentions != 0
        || raw.state_close_custody_retention_order != 0
        || raw.state_close_attempts != 0
        || raw.state_close_attempt_order != 0
        || raw.state_abandons != 0
        || raw.state_abandon_order != 0
    {
        return Err(anyhow!(
            "JointClose raw-state rejection did not fail before take/clear"
        ));
    }
    Ok(())
}

fn validate_raw_retry_delta(
    first: HandleBoundSqliteAbiRawCloseWitnessSnapshot,
    second: HandleBoundSqliteAbiRawCloseWitnessSnapshot,
) -> anyhow::Result<()> {
    if second
        .raw_close_entries
        .checked_sub(first.raw_close_entries)
        != Some(1)
        || second
            .state_take_attempts
            .checked_sub(first.state_take_attempts)
            != Some(1)
        || second.methods_clears.checked_sub(first.methods_clears) != Some(0)
        || second
            .state_take_successes
            .checked_sub(first.state_take_successes)
            != Some(0)
        || second.state_close_custody_retentions != 0
        || second.state_close_attempts != first.state_close_attempts
        || second.state_abandons != first.state_abandons
    {
        return Err(anyhow!(
            "JointClose raw-state saved-callback retry delta is not 1/1/0/0"
        ));
    }
    Ok(())
}
