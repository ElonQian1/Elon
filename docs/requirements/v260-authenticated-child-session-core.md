---
title: V260 authenticated child-only session core
status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
---

# V260 authenticated child-only session core

## Goal

Implement the first real ephemeral runtime component behind the V259 supervisor/session policy: a Linux x86-64 child-only `SOCK_SEQPACKET` session core that authenticates both endpoints from a one-time 32-byte seed, binds directional keys to the exact policy and runtime roots, and rejects malformed, unauthenticated, replayed, reflected, or out-of-order frames before releasing payload bytes.

## Acceptance criteria

- Create an anonymous Unix `SOCK_SEQPACKET | SOCK_CLOEXEC` pair and a separate anonymous one-way seed channel; no listener, pathname, TCP/UDP socket, DNS, TLS, or upstream handle is created.
- Generate seed and nonce material with the host operating-system CSPRNG and hold derived keys only in non-Clone, non-Debug, non-Serde zeroizing memory.
- Implement HKDF-SHA256 extract/expand with exact policy/profile/target/companion/capsule/bundle and both-nonce transcript binding, producing independent host-to-child and child-to-host keys.
- Complete a fixed-size mutual bootstrap proof before application frames are accepted; mismatched roots, seed, nonce, proof, length, magic, or version fail closed.
- Encode the V259 20-byte `ELSP` header and 32-byte HMAC-SHA256 tag over direction, transcript, header, and payload. Enforce known kinds, exact packet length, server-fixed limits, sequence starting at one, exact increment, no replay, no reflection, and no trailing bytes.
- Receive into a server-bounded buffer rather than allocating from an unauthenticated length field, and never expose a payload until constant-time MAC verification succeeds.
- Any protocol failure shuts down the endpoint and permanently rejects later send/receive attempts.
- Linux tests exercise real kernel socket and pipe behavior plus positive, tamper, replay, ordering, reflection, root/seed mismatch, oversize, and terminal-failure cases.

## Delivery boundary

- Keep the runtime core private to `compute_federation`; it is not an HTTP/MCP API and does not persist session material.
- Keep roots, crypto, transport, bootstrap, and Linux tests in focused sibling modules.
- Record executed commands, pass counts, evidence fingerprint, and exact residual gaps in dedicated authority and acceptance documents.

## Non-goals

- Do not fork, clone, spawn, `execveat`, execute the V257 capsule, remap descriptors to fd3/fd5, or implement pidfd shutdown/reap.
- Do not read or deliver V256 production config/credential bytes and do not create a reusable secret-delivery API.
- Do not implement namespace, cgroup, seccomp, rlimit, Landlock, AppArmor, mount, capability, or network enforcement in this batch.
- Do not perform DNS, TLS, upstream probe, Provider activation, routing, market admission, usage, verification, settlement, chain submission, deployment, or production credential access.
- Do not remove or reinterpret the V254 absolute deny fences or mark any V259 readiness flag true.
