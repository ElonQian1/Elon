use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::Path};

use anyhow::{bail, Result};

const WINDOWS_COMMAND_LINE_MAX_U16: usize = 32_767;

pub(super) fn nul_terminated_path(path: &Path) -> Result<Vec<u16>> {
    if !path.is_absolute() {
        bail!("COMPUTE_PLUGIN_WINDOWS_RUNNER_PATH_NOT_ABSOLUTE");
    }
    let mut value = path.as_os_str().encode_wide().collect::<Vec<_>>();
    if value.is_empty() || value.contains(&0) || value.len() >= WINDOWS_COMMAND_LINE_MAX_U16 {
        bail!("COMPUTE_PLUGIN_WINDOWS_RUNNER_PATH_INVALID");
    }
    value.push(0);
    Ok(value)
}

pub(super) fn command_line(application: &Path, arguments: &[String]) -> Result<Vec<u16>> {
    let mut output = quote_argument(application.as_os_str())?;
    for argument in arguments {
        output.push(u16::from(b' '));
        output.extend(quote_argument(OsStr::new(argument))?);
    }
    if output.len() >= WINDOWS_COMMAND_LINE_MAX_U16 {
        bail!("COMPUTE_PLUGIN_WINDOWS_RUNNER_COMMAND_LINE_TOO_LONG");
    }
    output.push(0);
    Ok(output)
}

/// Explicit empty Unicode environment. A future authenticated IPC/bootstrap contract must replace
/// this with a sorted allowlist; inheriting the Node process environment would leak secrets.
pub(super) const fn empty_environment_block() -> [u16; 2] {
    [0, 0]
}

fn quote_argument(value: &OsStr) -> Result<Vec<u16>> {
    let units = value.encode_wide().collect::<Vec<_>>();
    if units.contains(&0) {
        bail!("COMPUTE_PLUGIN_WINDOWS_RUNNER_ARGUMENT_CONTAINS_NUL");
    }
    let needs_quotes = units.is_empty()
        || units.iter().any(|unit| {
            *unit == u16::from(b' ') || *unit == u16::from(b'\t') || *unit == u16::from(b'"')
        });
    if !needs_quotes {
        return Ok(units);
    }

    let mut output = Vec::with_capacity(units.len() + 2);
    output.push(u16::from(b'"'));
    let mut backslashes = 0_usize;
    for unit in units {
        if unit == u16::from(b'\\') {
            backslashes += 1;
            continue;
        }
        if unit == u16::from(b'"') {
            output.extend(std::iter::repeat_n(
                u16::from(b'\\'),
                backslashes
                    .checked_mul(2)
                    .and_then(|count| count.checked_add(1))
                    .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_QUOTE_OVERFLOW"))?,
            ));
            output.push(unit);
        } else {
            output.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes));
            output.push(unit);
        }
        backslashes = 0;
    }
    output.extend(std::iter::repeat_n(
        u16::from(b'\\'),
        backslashes
            .checked_mul(2)
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_QUOTE_OVERFLOW"))?,
    ));
    output.push(u16::from(b'"'));
    Ok(output)
}
