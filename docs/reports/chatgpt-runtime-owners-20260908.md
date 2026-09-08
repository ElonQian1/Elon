# Committed runtime owner batch

Date: 2026-09-08. Base: `7723feede`. Adapter: 303.
Status: implemented, focused offline tests passed, published and installed in
APK 1567. Production stop acceptance did not pass; see the device result below.
This batch does not complete the overall private-native Goal.

## Changes

Reuse the existing `chatgpt_web_committed_owner_path.js` resolver in model
contract v4, regeneration contract v3, temporary chat v3 and attachment composer
v17. The previous independent walks trusted a return chain reaching
`root.current`, or imposed an arbitrary 90-parent limit. Attachment store
capture did not prove committed membership at all.

An orphan can still point at a current root without belonging to its children.
Conversely, an official React bailout can reuse a child whose return pointer
still names the old parent. The shared resolver proves actual child membership
through committed alternates, rejects ambiguous/cyclic ownership and bounds
all traversal. No new module loader, polling loop or document refresh is added.

- Model/effort actions use current eligible picker state, never stale disabled
  props from a previous render. Account, route and model restrictions remain.
- Regeneration resolves the current retry menu and retains original parent,
  model eligibility, write ownership and post-invocation no-replay rules.
- Temporary chat retains the exact observed official callback, cleanup and
  privacy readback. Saved temporary chats are not rewritten as ordinary chats.
- Attachment store and model/library policy use the same resolver. Orphaned
  stores cannot authorize byte upload or association. Existing MIME, model,
  project, temporary and library persistence policies are unchanged.

Stop runtime v3 also reuses text runtime v15's positively verified guest
identity. Guest support requires the official logged-out bootstrap and absent
live session, not merely absent credentials. Identity/proof, request and owner
are rechecked before dispatch. Voice states are still excluded. The existing
official stop transaction owns cancellation; no independent stop POST is added.

## Verification

Eleven new committed-owner cases failed against the preceding source. They
cover orphan rejection, reused-child current state and deep committed trees
across all four consumers. Existing synthetic fibers now model actual parent
child membership; ambiguous-menu tests still build genuinely committed graphs.

The first green run passed 341 Node tests across those contracts, models,
regeneration, temporary chat, attachment formats/scopes/reservations and runtime
integration. Five of six new guest-stop cases failed before the stop fix. The
second green run passed 56 tests across guest stop, existing stop/wiring,
current-runtime consumers and asset assembly. There were no failures, skips or
cancellations. These two suites overlap and are not 397 unique cases.

Log prefixes:

- `runtime-owner-red-20260908-20260908-125929-489`
- `runtime-owner-contract-tests-20260908-20260908-130513-667`
- `runtime-stop-guest-red-20260908-20260908-131054-854`
- `runtime-stop-guest-tests-20260908-20260908-131249-116`

## Device gate

One grouped Release build passed in 6m58s. Source commits: `ad0bba0fc` (committed
owners) and `9afd235e7` (guest stop). APK 1567 was published, remote bytes were
verified, and replacement installation on the trusted Xiaomi was confirmed.
APK size: 39,985,300 bytes; SHA-256:
`22b1a7b381ae0d620e074fef3fd3b3e20692a6c79049b7da32b7f5adea0089d2`.

Native MCP reported `social_ai`, adapter 303, guest mode, composer ready, no
draft, no streaming and zero messages. The isolated test sent a long synthetic
arithmetic prompt and observed a nonempty assistant response while streaming.
Stop then returned success, but its detail was empty, not the required
`official_runtime_v1:stop_observed`. The inspected adapter's DOM stop fallback
returns this empty detail. This therefore does not accept the private stop path.

The immediate stopped snapshot contained only one message and no nonempty
assistant text. Whether this is a transient projection gap or lost partial text
is not established. The 3,556 ms measurement includes command/poll latency and
is not a private-stop performance result. Follow-up sending was not attempted.
The restore commands for an empty conversation and home were accepted, but the
temporary script used PowerShell's read-only `HOME` name for the final readback
variable. That script error has been corrected, as has the follow-up receipt's
expected action (`send_prompt`). It did not cause the earlier stop result.

The phone subsequently locked. A read-only MCP attempt also lost service
availability; do not keep bootstrapping or replaying commands. On the next
unlocked round, inspect the in-flight capture/request/runtime guard that selected
fallback, then observe bounded post-stop snapshots before clearing the test
conversation. Do not repeat accepted search/text-send research.

Logs: `runtime-owner-release-20260908-20260908-132509-411`,
`runtime-owner-device-ready-20260908-20260908-133402-096`, and
`runtime-stop-1567-device-20260908-20260908-133638-090`.
Authenticated model, temporary, regeneration and attachment variants still need
an eligible existing session. Guest upsells do not remove those capabilities or
justify forcing login for basic chat. No thermal/completion claim is made.
