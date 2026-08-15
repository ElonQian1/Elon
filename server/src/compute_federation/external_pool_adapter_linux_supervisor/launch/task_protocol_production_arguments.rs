use std::ffi::CString;

use anyhow::Result;

use super::prefixed_digest_argument;

const ROOT_ARGUMENT_PREFIXES: [&str; 8] = [
    "--elon-task-production-policy=",
    "--elon-task-production-runtime-profile=",
    "--elon-task-production-protocol-profile=",
    "--elon-task-production-target=",
    "--elon-task-production-companion=",
    "--elon-task-production-launch-image=",
    "--elon-task-production-secret-delivery=",
    "--elon-task-production-conformance-receipt=",
];

pub(super) fn append(arguments: &mut Vec<CString>, values: &[String; 8]) -> Result<()> {
    for (prefix, value) in ROOT_ARGUMENT_PREFIXES.into_iter().zip(values) {
        arguments.push(prefixed_digest_argument(prefix, value)?);
    }
    Ok(())
}
