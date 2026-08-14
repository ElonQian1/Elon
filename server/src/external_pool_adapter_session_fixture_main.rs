//! Test-only sealed capsule for V262 Linux kernel acceptance.
//!
//! The binary receives only non-sensitive root digests in fixed argv positions and the V261
//! inherited fd3/fd5 topology. It has no secret resolver, upstream transport, Provider control
//! plane, persistence, HTTP, MCP, or economic effect.

#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use std::time::Duration;

use anyhow::{bail, Context, Result};
use elon_external_pool_adapter_session_core::{
    execute_external_pool_adapter_no_work_probe,
    receive_external_pool_adapter_ephemeral_bundle_from_begin, ExternalPoolAdapterChildBootstrap,
    ExternalPoolAdapterSessionFrameKind, ExternalPoolAdapterSessionRoots,
};

const HOST_READY: &[u8] = b"v262.host.authenticated";
const CHILD_READY: &[u8] = b"v262.child.authenticated";
const SHUTDOWN: &[u8] = b"v262.shutdown";
const V263_CONFIG: &[u8] = br#"{"mode":"test-no-work"}"#;
const V265_CONFIG: &[u8] = br#"{"mode":"test-upstream-no-work"}"#;
const V263_CREDENTIAL: &[u8] = b"test-credential-never-production";
const V265_REQUEST: &[u8] = b"ELON-TEST-NO-WORK\n";
const V265_RESPONSE: &[u8] = b"ELON-TEST-NO-TASK\n";
const V265_PROBE_TIMEOUT: Duration = Duration::from_millis(15_000);
const ROOT_ARGUMENT_PREFIXES: [&str; 6] = [
    "--elon-session-policy=",
    "--elon-session-profile=",
    "--elon-session-target=",
    "--elon-session-companion=",
    "--elon-session-capsule=",
    "--elon-session-bundle=",
];

fn main() {
    if let Err(error) = run() {
        eprintln!("external-pool Adapter session fixture failed: {error:#}");
        std::process::exit(111);
    }
}

fn run() -> Result<()> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() != ROOT_ARGUMENT_PREFIXES.len() + 1
        || arguments[0] != "elon-external-pool-adapter"
    {
        bail!("fixed supervisor argv contract rejected");
    }
    let (roots, bundle_root) = parse_roots(&arguments[1..])?;
    let child = unsafe { ExternalPoolAdapterChildBootstrap::adopt_supervisor_descriptors() };
    let mut session = child
        .authenticate(roots)
        .context("authenticate inherited supervisor session")?;

    let first = session
        .receive()
        .context("receive first authenticated runtime frame")?;
    if first.kind() == ExternalPoolAdapterSessionFrameKind::Control && first.payload() == HOST_READY
    {
        session
            .send(ExternalPoolAdapterSessionFrameKind::Control, CHILD_READY)
            .context("send authenticated child-ready frame")?;
        require_control(&mut session, SHUTDOWN)?;
        return Ok(());
    }
    let delivered = receive_external_pool_adapter_ephemeral_bundle_from_begin(
        &mut session,
        &bundle_root,
        first,
    )
    .context("receive V263 ephemeral bundle")?;
    if (delivered.config() != V263_CONFIG && delivered.config() != V265_CONFIG)
        || delivered.credential() != V263_CREDENTIAL
    {
        bail!("V263 test-only material rejected");
    }
    if delivered.config() == V265_CONFIG {
        execute_external_pool_adapter_no_work_probe(
            &mut session,
            V265_REQUEST,
            V265_RESPONSE.len(),
            V265_PROBE_TIMEOUT,
            |response| {
                if response != V265_RESPONSE {
                    bail!("V265 test-only no-work response rejected");
                }
                Ok(())
            },
        )
        .context("execute V265 authenticated no-work probe")?;
    }
    delivered
        .wait_for_shutdown(&mut session)
        .context("zeroize V263 delivery and acknowledge shutdown")?;
    Ok(())
}

fn parse_roots(arguments: &[String]) -> Result<(ExternalPoolAdapterSessionRoots, String)> {
    if arguments.len() != ROOT_ARGUMENT_PREFIXES.len() {
        bail!("fixed root argument count rejected");
    }
    let values: Vec<&str> = arguments
        .iter()
        .zip(ROOT_ARGUMENT_PREFIXES)
        .map(|(argument, prefix)| {
            argument
                .strip_prefix(prefix)
                .ok_or_else(|| anyhow::anyhow!("fixed root argument prefix rejected"))
        })
        .collect::<Result<_>>()?;
    Ok((
        ExternalPoolAdapterSessionRoots::new(
            values[0], values[1], values[2], values[3], values[4], values[5],
        )?,
        values[5].to_string(),
    ))
}

fn require_control(
    session: &mut elon_external_pool_adapter_session_core::AuthenticatedExternalPoolAdapterSession,
    expected: &[u8],
) -> Result<()> {
    let frame = session
        .receive()
        .context("receive authenticated control frame")?;
    if frame.kind() != ExternalPoolAdapterSessionFrameKind::Control || frame.payload() != expected {
        bail!("authenticated control frame rejected");
    }
    Ok(())
}
