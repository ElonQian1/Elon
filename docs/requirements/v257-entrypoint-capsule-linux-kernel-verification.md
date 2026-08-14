---
title: V257 entrypoint capsule Linux kernel verification
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
---

# V257 entrypoint capsule Linux kernel verification

## Goal

Execute the existing V257 entrypoint-capsule materialization path on Linux x86-64 and bind its claims to real kernel behavior. The batch must verify anonymous memfd creation, exact executable permissions, immutable seals, byte identity, fail-closed source validation, and descriptor release without launching the capsule.

## Acceptance criteria

- The `elon-server` test target compiles under WSL2 Ubuntu x86-64 with the Linux capsule implementation enabled.
- A positive fixture creates a zero-link anonymous memfd with exact mode `0500`, `FD_CLOEXEC`, exact size/SHA-256, and exactly `F_SEAL_WRITE | F_SEAL_GROW | F_SEAL_SHRINK | F_SEAL_SEAL`.
- Kernel write, grow, shrink, and seal-set mutation attempts fail after materialization.
- Invalid source mode, extra hard link, incorrect digest/size, and unsafe ELF input fail closed before any consumer effect.
- The borrowed callback cannot retain ownership; after it returns, the observed capsule descriptor is closed.
- No `execveat`, process, IPC/session, secret delivery, DNS/TLS/network, probe, route, activation, market, usage, settlement, production credential, or deployment is introduced or executed.

## Delivery boundary

- Add Linux-only tests beside the private V257 capsule implementation.
- Use generated non-production ELF fixture bytes and temporary local files only.
- Record exact command, environment, pass count, evidence fingerprint, and remaining production gaps in current authority documents.

## Non-goals

- Do not execute the generated ELF or implement the future supervisor.
- Do not create authenticated child-only IPC, session keys, KDF output, transcripts, or upstream sockets.
- Do not claim production kernel, mount, ACL, cgroup, namespace, seccomp, Landlock, AppArmor, secret custody, or Provider readiness.
- Do not remove or reinterpret the V254 absolute deny fences.
