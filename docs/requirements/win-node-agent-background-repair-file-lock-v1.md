# Win node-agent background repair file-lock recovery v1

## Problem

The post-terminal activator can start a verified package while the installed node runtime or watchdog still maps `一龙开发平台.exe`. The background repair currently ignores process-stop failures and performs a single copy attempt. Windows then returns sharing violation (`os error 32`), and the rollback repair can fail for the same reason.

## Scope

- Win node-agent launcher and post-terminal activation only.
- Preserve the existing safety gate that refuses to close a visible desktop shell during background repair.
- Do not modify the PWA.

## Acceptance criteria

1. Background repair proves all replaceable installed launcher processes have exited before copying the new executable.
2. Process enumeration or termination failures fail closed with actionable launcher-log detail.
3. A bounded retry handles transient Windows sharing violations without hiding permanent failures.
4. The installed runtime and watchdog recover on the exact published release identity.
5. Existing launcher, local-first release, Rust, and Win Google AI regression tests pass.
