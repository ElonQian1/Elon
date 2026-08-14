---
title: V259 supervisor session policy companion dynamic verification
version_status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
---

# V259 supervisor session policy companion dynamic verification

## Goal

Compile and execute the existing V259 inert supervisor/session policy companion implementation. Fix only defects exposed by compilation or focused tests, and leave a maintainable acceptance record for migration, Store, owner/admin HTTP, redaction, lineage, currentness, revocation, and no-side-effect boundaries.

## Required Evidence

- The full `elon-server` test target compiles with the V259 Domain, migration, Store, Service, and API modules enabled.
- Focused tests execute fresh and repeat migration, frozen policy and persistence contracts, exact V258/V255 lineage, currentness, revocation, and V254 fence compatibility.
- Owner/admin HTTP tests execute authentication, authorization, strict input validation, actor-bound replay, linear recovery, redaction, and inert database effects.
- Source contracts prove the V259 path has no process, IPC, secret delivery, DNS, TLS, network, probe, activation, route, market, usage, settlement, MCP, or PC side effect.
- Current documentation records the actual test count and validation fingerprint without upgrading untested runtime or production claims.

## Non-goals

- Do not use production mounts, credentials, certificates, DNS, sockets, remote services, platform APIs, external payments, Sui networks, or mainnet assets.
- Do not start an Adapter, Sidecar, supervisor, child process, authenticated session, child-only IPC channel, or no-work probe.
- Do not activate a Provider, remove or weaken the V254 absolute market fences, or create route, service actor, workload, usage, or settlement effects.
- Do not deploy the server or claim Linux syscall, production database, concurrency, crash-recovery, or public-network acceptance unless separately executed.

## Implementation Scope

- `server/src/compute_federation/external_pool_adapter_supervisor_session_policy_companion*`
- `server/src/compute_federation/external_pool_adapter_release_api_tests/supervisor_session_policy_companion_*`
- `server/src/store/compute_external_pool_adapter_supervisor_session_policy_companion/`
- `server/src/store_migrations/compute_external_pool_adapter_supervisor_session_policy_companion*`
- V259 authority, acceptance, and current-status documents
