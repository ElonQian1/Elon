---
title: "Rust cache portable file hashing V1"
owner: platform
status: implemented_locally_verified
reviewed_at: 2026-08-16
implementation_refs:
  - file:scripts/rust-cache/RustCache.Portability.psm1
  - file:scripts/rust-cache/RustCache.Fleet.psm1
  - file:scripts/rust-cache/RustCache.FleetQueue.psm1
  - file:scripts/test-rust-cache-portability.ps1
---

# Rust cache portable file hashing V1

## Problem

Fleet report and outbox export used the PowerShell `Get-FileHash` command directly. A valid Windows PowerShell host can run without that command being auto-loaded, causing otherwise portable cache installation and reporting tests to fail.

## Requirements

1. Cache platform file hashing must use the .NET runtime available to both Windows PowerShell 5.1 and PowerShell 7.
2. Fleet report and immutable outbox exports must return lowercase SHA-256 values with unchanged public schemas.
3. The shared helper must stream file contents instead of loading large reports entirely into memory.
4. The change must not alter cache routing, partition locks, GC policy, report privacy, or destructive-operation authority.
5. Existing portability tests must pass without requiring profile scripts or PowerShell module auto-loading.

## Acceptance

- `scripts/test-rust-cache-portability.ps1` passes from a no-profile hidden Windows PowerShell process.
- `scripts/test-rust-cache-platform.ps1` passes.
- No production cache module calls `Get-FileHash`.
