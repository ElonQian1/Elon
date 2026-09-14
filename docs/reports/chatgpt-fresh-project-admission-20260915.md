# Project Send Admission Follow-Up

Status: adapter 413 is published/installed as 1.1.1750; its read-only probe passed.
Adapter 414 adds the evidence-backed ordinary GLOBAL-scope compatibility fix;
its Android/package validation and post-install admission are pending.
Production project-independent sends remain disabled. Personal fresh text,
composer-free first sends and local attachment defaults are unchanged.

## Actual Device Result

`fresh-project-native-1749-20260915-071601-052` exercised the production native
Send button once, in an existing synthetic project conversation. Exact known
test messages, a fresh file index and project membership were verified first.

- Independent sender: one attempt, `scope_unsupported`, zero POSTs and zero
  stream events. This is a failed independent-project acceptance, not success.
- Existing official-runtime fallback: one accepted send, one exact synthetic
  user message and one completed native reply. Read-only follow-up verified
  project membership. No replay, new project or second test send.
- The unknown-write ledger was resolved from actual readback and archived as
  `official_runtime_fallback`, with `private_dispatch_verified=false`.
  The original conversation and awake settings were restored.

The installed 1750 probe on the same owned fixture returned
`scope_unsupported / base_project`, with zero Send clicks, an empty draft and
no stream. Original route and awake state were restored. This narrows the
failure but does not identify one individual predicate in the old grouped guard.

## Implementation

`chatgpt-fresh-project-smoke.ps1` reuses the existing native-send acceptance,
not another sender. It checks exact test content rather than a title alone,
matches request-specific file receipts, and permits one retired read retry.
Project routes, preserved earlier users and duplicate messages are checked.

Earlier pre-send failures exposed acceptance assumptions, not failed sends:
the first title candidate was not the media fixture; the correct fixture had
an earlier known citation test; a navigation retired a file read; and the
phone's clock was about two seconds behind the PC. Membership freshness now
compares phone receipts to the previous phone receipt, not a PC timestamp.
Every rejected preflight sent zero messages. Historical resolved personal
write evidence was rechecked and archived without deleting server data.

`fresh_text_admission` adds a bounded, read-only production MCP probe using
the same context capture as a send. It reports only allowlisted `code` and
`stage`, including project business/header checks. It does not arm a trial,
prepare or POST, mutate the draft, start a stream or export private values.
The adapter only routes the command; the existing transaction orchestrator
owns inspection. No diagnostic panel or duplicate user-facing control is added.

## Ordinary Project Scope Fix

Reviewed September 15 public code hydrates `context_scopes` into the selected
thread's `contextScopes`; shared `$2` explicitly treats `GLOBAL` as ordinary
context. The old native guard rejected every nonempty array. A new behavior
test reproduced that incompatibility before the fix. Context v20 permits only
null/empty or all-`GLOBAL` arrays; malformed, sparse, HEALTH, LOCKED_CHATS and
unknown scopes remain rejected, including scope changes after capture.
Project identity, owner, mode, privacy, business-agent and writer guards remain.
The readonly stages now distinguish route/mode/loading/privacy/shared/scopes,
so an unrelated remaining guard cannot be mislabeled as the GLOBAL defect.
Project dispatch stays trial-only until an actual independent request succeeds.

## Verification

- `fresh-project-admission-regression-20260915-072724-234`: 378 Node tests
  passed, zero failures/skips, including actual context inspection and wiring.
- Project acceptance guards: 34 path/content/file/continuity/membership cases
  plus eight fallback-readback cases passed. Existing send, cleanup and
  uncertain-write evidence tests remain required before commit.
- Adapter 413: 13 Android unit tests passed, zero failures/skips;
  `fresh-project-admission-android-unit-20260915-072756-256`.
- Normal Release 1750 from `ed244c24bfc6178ef3643bb8b72fdfa0e6755c4c`
  built, passed remote version/size/hash checks and installed without data reset.
  SHA-256: `2f869f6287246ea9bc294fa922ee61a48d22fb4a6f853729be17f9ade60f9780`.
  `fresh-project-admission-release-20260915-073824-760` passed in 466s.
- GLOBAL fix: `fresh-project-global-regression-20260915-075355-096` passed
  379 Node cases. Hash-pinned public-source AST checks passed all three cases
  with no skip (`fresh-project-global-public-contract-20260915-075400-018`).
- Adapter 414 Android unit/Release/read-only admission remain pending.

## Remaining

Read admission on adapter 414 before any new project send. Only if all actual
guards pass, perform one new scoped native acceptance, never replay the prior
fallback-accepted message. Do not loosen project ownership for acceptance.
Original editable Canvas acceptance still needs an actual eligible original
document; Writing Blocks and synthetic code blocks do not prove that loop.

Related: [project scope](../chatgpt-fresh-project-text-dispatch.md),
[current runtime batch](chatgpt-runtime-bindings-20260915.md).
