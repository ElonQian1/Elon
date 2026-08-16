---
name: manage-shared-build-cache
description: Install, diagnose, operate, and safely reclaim the Elon portable Rust build-cache platform across repositories, worktrees, Windows PCs, and remote development nodes. Use when Rust builds create duplicate target directories, disks fill with Cargo artifacts, multiple AI agents share one machine, a project needs rust-cache.project.json adoption, a cache installation may be stale, or cache cleanup must preserve active, dirty, unknown, or external workspaces.
---

# Manage Shared Build Cache

Use the repository cache tool as the source of truth. Do not reproduce cache routing or deletion logic inside the skill.

## Locate The Tool

1. Prefer `<project>/scripts/rust-cache.ps1` when it exists.
2. Otherwise use `%LOCALAPPDATA%\Elon\bin\rust-cache.ps1`, the stable per-user launcher written by the installer.
3. If the launcher is absent but `ELON_RUST_CACHE_ROOT` is set, use `$env:ELON_RUST_CACHE_ROOT/platform/rust-cache.ps1` and repair the launcher with a current installer.
4. If none exists, obtain a current trusted checkout of the platform repository and run its installer.
5. Invoke the script directly from the current PowerShell session. Do not open nested visible `powershell.exe` or `pwsh.exe` windows.
6. Run `& <entry> help` when command availability or parameter intent is unclear.

## Diagnose First

Run:

```powershell
& <entry> doctor -ProjectRoot <absolute-project-root>
```

Treat exit code `2` as an actionable health report, not as an unknown tool crash. Read every failed check before changing the machine. Report the resolved cache root, project ID, source/install fingerprint state, active Cargo writers, disk state, and recommended command.

## Adopt A Project

Keep machine paths out of Git. Preview the portable manifest first:

```powershell
& <entry> init-project -ProjectRoot <root> -ProjectId <stable-slug> `
  -AllowedDomain dev-windows-msvc,agent-validation
```

Review the JSON, then repeat with `-Apply`. Commit `rust-cache.project.json` with the project so every PC uses the same project identity and domain policy. Add named shared partitions only for stable, reviewed workflows.

## Install Or Upgrade A PC

From a current trusted platform checkout, run:

```powershell
& .\scripts\rust-cache.ps1 install -ProjectRoot . -Apply -InstallCodexSkill
& .\scripts\rust-cache.ps1 doctor -ProjectRoot .
```

The installer writes a canonical source fingerprint and an exact installed-byte fingerprint. Equivalent LF/CRLF or UTF-8 BOM checkout forms identify the same reviewed source, while any post-install file change still fails integrity checks. Re-run the installer when `doctor` reports genuine platform drift or installed-file tampering. Installing the Codex skill is per user and per PC; project manifests remain portable Git files.
The installer also writes `%LOCALAPPDATA%\Elon\bin\rust-cache.ps1`. Child repositories call this launcher instead of copying platform modules or hard-coding a machine cache root.
The installer serializes upgrades on each PC. Wait for the current installer instead of copying platform files manually.

## Use Across A PC Fleet

- Install or upgrade the platform and Skill independently on every PC from the same trusted Git revision.
- Commit only `rust-cache.project.json` and thin project wrappers. Do not commit cache roots, launcher targets, user profiles, or node data paths.
- Let Git distribute project identity and domain policy; let each PC choose its own physical cache volume.
- Run `doctor` on each PC before accepting a long build. A healthy PC must report matching canonical source identity, exact installed-byte integrity, Codex Skill integrity, and a healthy user launcher.
- Central dashboards may collect read-only doctor/status reports. GC `-Apply`, Cargo parent-config activation, legacy purge, and cache migration remain machine-local reviewed operations.

Generate the standard sanitized fleet artifact on each node:

```powershell
& <entry> fleet-report -ProjectRoot <root> -NodeId <platform-node-id> -IncludeSizes
```

Use the platform node ID supplied by the owning system; do not invent identity from a host name or user profile. The report intentionally omits absolute paths and user/host names. A central service may aggregate it and request a dry-run, but deletion must be re-evaluated and executed on the target PC under the local partition locks.

For a disconnected or intermittently connected node, stage an upload-safe envelope instead:

```powershell
& <entry> fleet-stage -ProjectRoot <root> -NodeId <platform-node-id> -IncludeSizes
```

`fleet-stage` writes an immutable envelope under the cache-owned fleet outbox. The envelope embeds the compact sanitized report, its SHA-256, an explicit requirement that the receiver authenticate the node, and a false destructive-authority flag. Never edit an envelope to record retries; a node uploader must write separate attempt receipts and move an accepted envelope only after the server acknowledges the same envelope ID and report hash.

## Run Builds

- Prefer the project's `cargo-dev.ps1`, `cargo-cross.ps1`, validation, and release scripts.
- Use `rust-cache.ps1 run` only when no narrower project wrapper exists.
- Let the manifest determine project ID and domain. Never invent function-, session-, PID-, or agent-specific shared partitions.
- Reuse named shared partitions only inside one registered project and compatible domain.
- Use SCCache for object reuse across projects; do not share one raw Cargo target directory across unrelated projects.
- Before starting a long build, check whether an equivalent Cargo/rustc process is already running. Wait for it instead of launching a duplicate.

## Reclaim Space

1. Run `status -IncludeSizes` and `gc` without `-Apply`.
2. Inspect the generated report, active writers, locks, ownership, workspace existence, and legacy-cache registration.
3. Apply only the reviewed managed action.
4. Register external legacy caches and mark them retired before any purge attempt.

Never use raw recursive deletion on shared cache roots. Never delete active, locked, dirty, unmerged, unknown, or external workspace data. Never pass `-ForceAged` merely to silence a low-disk warning.

## Close The Work

Report:

- tool entry and source hash;
- cache root, project ID, domain, scope, and shared partition;
- whether the operation was preview or apply;
- active writers and locks encountered;
- reclaimed bytes and report path;
- checks or builds actually run;
- anything preserved or not verified.

For policy and architecture details, read `docs/rust-cache-platform.md` and `docs/rust-cache-on-demand-adoption.md` from the platform repository. Do not duplicate those documents into child projects.
For multi-PC rollout, report aggregation, and the GC approval state machine, read `docs/rust-cache-fleet-operations.md`.
