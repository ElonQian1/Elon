---
name: manage-shared-build-cache
description: Install, diagnose, operate, and safely reclaim the Elon portable Rust build-cache platform across repositories, worktrees, Windows PCs, and remote development nodes. Use when Rust builds create duplicate target directories, disks fill with Cargo artifacts, multiple AI agents share one machine, a project needs rust-cache.project.json adoption, a cache installation may be stale, or cache cleanup must preserve active, dirty, unknown, or external workspaces.
---

# Manage Shared Build Cache

Use the repository cache tool as the source of truth. Do not reproduce cache routing or deletion logic inside the skill.

## Locate The Tool

1. Prefer `<project>/scripts/rust-cache.ps1` when it exists.
2. Otherwise use `$env:ELON_RUST_CACHE_ROOT/platform/rust-cache.ps1`.
3. If neither exists, obtain a current trusted checkout of the platform repository and run its installer.
4. Invoke the script directly from the current PowerShell session. Do not open nested visible `powershell.exe` or `pwsh.exe` windows.

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

The installer writes a source fingerprint. Re-run it when `doctor` reports platform drift. Installing the Codex skill is per user and per PC; project manifests remain portable Git files.
The installer serializes upgrades on each PC. Wait for the current installer instead of copying platform files manually.

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
