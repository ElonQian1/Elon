use std::ffi::CString;

use anyhow::Result;

use super::prefixed_digest_argument;

const ROOT_ARGUMENT_PREFIXES: [&str; 14] = [
    "--elon-task-protocol-conformance-session-policy=",
    "--elon-task-protocol-conformance-profile=",
    "--elon-task-protocol-conformance-run-nonce=",
    "--elon-task-protocol-conformance-fixture-catalog=",
    "--elon-task-protocol-conformance-registry-release=",
    "--elon-task-protocol-conformance-installation-content=",
    "--elon-task-protocol-conformance-capability-set=",
    "--elon-task-protocol-conformance-sandbox-reattestation-receipt=",
    "--elon-task-protocol-conformance-runtime-compatibility-receipt=",
    "--elon-task-protocol-conformance-source-capsule=",
    "--elon-task-protocol-conformance-launch-image=",
    "--elon-task-protocol-conformance-public-delivery=",
    "--elon-task-protocol-conformance-synthetic-fixture-lane=",
    "--elon-task-protocol-conformance-synthetic-fixture-executor=",
];

pub(super) fn append(arguments: &mut Vec<CString>, values: &[String; 14]) -> Result<()> {
    for (prefix, value) in ROOT_ARGUMENT_PREFIXES.into_iter().zip(values) {
        arguments.push(prefixed_digest_argument(prefix, value)?);
    }
    Ok(())
}
