# September 15 Second Rollout

Status: bindings 30 / adapter 416 implemented and offline verified; normal
Release and native project acceptance pending. No new capability is promoted.

## Observed Boundary

Normal 1752 / adapter 415 built, published and installed from `3db3240c4`.
SHA-256: `3cd815d472f42629026592363df4a73fec65e7e72d2e754cb85ac03eb01ba3e5`.
`fresh-project-owner-release-20260915-081859-780` passed in 445.2s.

The same owned project fixture returned `runtime_unavailable / base_context`,
with zero Send clicks, empty draft, no stream and successful route/awake
restoration. A separate read-only runtime-assets probe found the new anchor
below. Thus this result does not prove a failure or success of the owner fix;
the installed profile was no longer the live official revision. No message
was replayed and the earlier runtime-fallback send is not an independent pass.

## Exact Public Contracts

Files were downloaded without account headers, parsed, never executed.
Artifacts remain outside the task tree under `runtime-20260915-b`.

| Role | File | SHA-256 |
|---|---|---|
| Anchor | `c2675c8c-hyq5rrycw8ynuek2.js` | `13090b4a8f5cf432ec3694a4ab7ed214ece5498171bddf58e4e9708660d3f96e` |
| Shared | `4813494d-npmn21nlnk5b1x5g.js` | `318a749ff4dfcc37b182d7895b87072387f9d7c9fa0d450cc6bad9e3862bee94` |
| Conversation | `conversation-small-newrvr7nrx5tnmp4.js` | `039efa3e391652942a9ae5de7cc057eb1bc05c3afad32d851e14ac47b554470d` |
| Composer | `8b34dbc2-duz5rkq42xhpbotn.js` | `5693a37d5f6eecdecd6bb9e257380cc7594803aeafb9df87b5dbafdf4d03f605` |
| React | `2340486e-lo737pyjfygyimqo.js` | `a0e3af9dc19a43aff7b55fe57a6ebca9f8ce108eb599880091abbec880551e9d` |

All 100 canonical exports retain their actual function/store identity through
the existing loader. Normalized bodies are not the sole evidence: ambiguous
status, account, PIN, WEB-prefix and library-session getters have explicit
dependency checks; React bytes are identical. Temporary action and Canvas
export/generation imports match this exact revision and reject profile changes.

The stop function changed: exported `HWt` now obtains a current conduit token,
passes it only inside the page, clears it only if still the same token, and
aborts the exact active request. Native runtime stop still calls the official
function with the same arguments; no copied stop request or credential bridge
is introduced. The independent fresh sender retains its existing owned stop
token contract. Real stop acceptance on this revision remains separate.

History pending-draft/identity logic and six-argument attachment serialization
retain their reviewed shapes. Existing modules are reused, including the
personal first-send and attachment defaults. Project sends remain trial-only.

## Verification

- Before the profile patch, the new loader test failed `runtime_not_observed`;
  both public-source contract tests already passed.
- `runtime-sep15b-contract-20260915-084125-017`: three tests passed, no skips;
  hash-pinned AST, 100 production loader aliases, Canvas import/restore contracts.
- `runtime-sep15b-regression-20260915-084212-263`: 1043 passed, four optional
  evidence skips, zero failures. Covers fresh sends, owner changes, tools,
  attachments, history, stop/recovery, Canvas and Writing Blocks.
- No new project Send or original Canvas mutation has been accepted yet.

Next: install this exact normal Release, read project admission, and only on
`ready` perform one new native project Send. Keep existing resolved ledgers;
never replay a prior message. Original Canvas still requires an eligible owned
document, not a synthetic code block or Writing Block.
