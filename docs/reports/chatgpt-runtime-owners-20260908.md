# Committed runtime owner batch

Date: 2026-09-08. Base: `7723feede`. Adapter: 303.
Status: implemented and focused offline tests passed; production acceptance is
pending. This batch does not complete the overall private-native Goal.

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

Build and install the batch once, then exercise production native controls.
Guest stop is independently testable without logging in; authenticated model,
temporary, regenerate and attachment variants require an eligible existing
website session. Do not treat guest login upsells as missing capabilities or
force login for ordinary chat. Preserve cookies, drafts and original surface.
No latency, thermal or full capability-completion claim follows from fixtures.
