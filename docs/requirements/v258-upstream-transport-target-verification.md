---
title: V258 upstream transport target dynamic verification
version_status: current
reviewed_at: 2026-08-14
owners: backend, security, ai-economy
---

# V258 upstream transport target dynamic verification

## Goal

Compile and execute the existing V258 inert upstream transport target implementation. Fix only defects exposed by compilation or focused tests, and leave a maintainable acceptance record for migration, Store, owner/admin HTTP, redaction, lineage, currentness, revocation, and no-side-effect boundaries.

## Required Evidence

- The full `elon-server` test target compiles with V258 Domain, migration, Store, Service, and API modules enabled.
- Focused tests execute the fresh/repeat migration and frozen persistence, policy, projection, root, lineage, timestamp, immutability, and V254 fence contracts.
- Owner/admin HTTP tests execute authentication, authorization, strict JSON, canonical target validation, actor-bound replay, linear successor, currentness, revocation, redaction, and inert database effects.
- Source contracts prove the V258 path has no DNS, TLS, socket, process, secret delivery, probe, activation, route, market, usage, settlement, HTTP client, MCP, or PC side effect.
- Current documentation records the actual test count and validation fingerprint without upgrading untested runtime or production claims.

## Non-goals

- Do not use production mounts, credentials, certificates, DNS, sockets, remote services, platform APIs, external payments, Sui networks, or mainnet assets.
- Do not start an Adapter, Sidecar, supervisor, child process, authenticated session, or no-work probe.
- Do not activate a Provider, remove or weaken the V254 absolute market fences, or create route, service actor, workload, usage, or settlement effects.
- Do not deploy the server or claim Linux, production database, concurrency, crash-recovery, or public-network acceptance unless separately executed.

## Implementation Scope

- `server/src/compute_federation/external_pool_adapter_upstream_transport_target*`
- `server/src/compute_federation/external_pool_adapter_release_api_tests/upstream_transport_target_*`
- `server/src/store/compute_external_pool_adapter_upstream_transport_target/`
- `server/src/store_migrations/compute_external_pool_adapter_upstream_transport_target*`
- V258 authority, acceptance, and current-status documents
