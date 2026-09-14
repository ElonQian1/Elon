# Project Send Admission Follow-Up

Status: implementation and offline checks complete; Android validation pending.
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

## Verification

- `fresh-project-admission-regression-20260915-072724-234`: 378 Node tests
  passed, zero failures/skips, including actual context inspection and wiring.
- Project acceptance guards: 34 path/content/file/continuity/membership cases
  plus eight fallback-readback cases passed. Existing send, cleanup and
  uncertain-write evidence tests remain required before commit.
- Android unit test, Release and post-install read-only admission are pending.
  Adapter target is 413; this does not claim a published version yet.

## Remaining

Read the exact rejection stage on-device, compare it to the observed provider
contract, and correct only a proven compatibility mismatch. Do not loosen
project ownership or send another prompt merely to obtain diagnostics.
Original editable Canvas acceptance still needs an actual eligible original
document; Writing Blocks and synthetic code blocks do not prove that loop.

Related: [project scope](../chatgpt-fresh-project-text-dispatch.md),
[current runtime batch](chatgpt-runtime-bindings-20260915.md).
