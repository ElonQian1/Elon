//! Test-only sealed capsule for V262 Linux kernel acceptance.
//!
//! The binary receives only non-sensitive root digests in fixed argv positions and the V261
//! inherited fd3/fd5 topology. It has no secret resolver, upstream transport, Provider control
//! plane, persistence, HTTP, MCP, or economic effect.

#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use anyhow::{bail, Context, Result};
use elon_external_pool_adapter_session_core::{
    ExternalPoolAdapterChildBootstrap, ExternalPoolAdapterSessionFrameKind,
    ExternalPoolAdapterSessionRoots,
};

const HOST_READY: &[u8] = b"v262.host.authenticated";
const CHILD_READY: &[u8] = b"v262.child.authenticated";
const SHUTDOWN: &[u8] = b"v262.shutdown";
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
    let roots = parse_roots(&arguments[1..])?;
    let child = unsafe { ExternalPoolAdapterChildBootstrap::adopt_supervisor_descriptors() };
    let mut session = child
        .authenticate(roots)
        .context("authenticate inherited supervisor session")?;

    require_control(&mut session, HOST_READY)?;
    session
        .send(ExternalPoolAdapterSessionFrameKind::Control, CHILD_READY)
        .context("send authenticated child-ready frame")?;
    require_control(&mut session, SHUTDOWN)?;
    Ok(())
}

fn parse_roots(arguments: &[String]) -> Result<ExternalPoolAdapterSessionRoots> {
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
    ExternalPoolAdapterSessionRoots::new(
        values[0], values[1], values[2], values[3], values[4], values[5],
    )
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
