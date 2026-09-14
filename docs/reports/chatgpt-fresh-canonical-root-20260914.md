# First-Send Canonical Root Reconciliation

## Current Evidence

The 1718 native first-send trial `fresh-new-store1718-20260914-091231-662`
failed in 105.3 seconds. Exactly one native Send click dispatched one accepted
private request and received 31 stream events. Account, document, route and
conversation ownership remained valid; the precise failure was
`store_parent_mismatch`, after the network history had been verified.
No follow-up, replay or automatic fallback send occurred. The uncertain fixture
and its observed provider route remain in the local acceptance handoff. The
awake setting was restored; conversation restoration was deliberately deferred.

## Root Cause And Fix

The pinned `web_20260912` shared bundle exports `yi/VHt`. It applies paginated
messages to the existing `Cx` tree, using that tree's `rootId`, rather than
installing the response's `paginated-root:<conversation>` node. Conversation
`fy` selects this branch for paginated history when an existing `Cx` is present.
The previous test helper replaced the whole tree, so it missed this distinction.

Fresh reconcile v12 accepts the original `client-created-root` only for the
already verified, complete, conversation-bound pagination response when its
server root is absent from the canonical tree. Every hidden ancestor identity,
parent, reciprocal child link, root role and message identity is still checked.
Full-history roots and unrelated canonical roots receive no exception. Requests,
security checks, admission scopes and uncertain-write replay rules are unchanged.
Adapter 388 delivers the repair; new-conversation default remains trial-only
until native first-send and follow-up acceptance.

## Verification

- `fresh-retained-root-baseline-20260914-092029-164` reproduced the first-send
  failure after the test helper was changed to preserve the existing root.
- The helper now returns real parent nodes and correctly normalizes the legacy
  empty root, instead of returning synthetic parent IDs for missing nodes.
- `fresh-retained-root-full-20260914-092249-260` passed 286 Node tests, zero
  failures or skips. Coverage includes first send/follow-up, private streams,
  stop, projects, tools, temporary conversations, attachments, regeneration,
  malformed or changed canonical ancestry, and pinned public-source contracts.
- Public shared/conversation/composer hashes are asserted by
  `test-chatgpt-text-dispatch-public-evidence.cjs`; bundles are parsed, not executed.

## 1719 Native Recheck

Release 1.1.1719 (`84f4abba2`) compiled, published and installed without clearing
data. `fresh-retained-root-readback-20260914-093404-871` resolved the prior exact
fixture with zero sends. `fresh-retained-root-native-20260914-093521-364` then
made exactly one native first-send click. The request was accepted, all 31 stream
events arrived, canonical history reconciled, and the writer completed. The
original root mismatch is fixed on device, not merely in a substitute tree.

The full UI acceptance still failed: native Send allocated an ID but did not
register it in the observable command ledger, and the cached stream placeholder
was retained beside its canonical answer. Read-only inspection verified the
same provider message UUID and equivalent escaped text, not two server writes.
The handoff is preserved for zero-send recovery; no follow-up or replay occurred.

The next native patch registers the already allocated send ID immediately before
transport dispatch, reusing existing MCP registrations. Snapshot merging keys
only exact assistant `private-stream:<UUID>` aliases to the provider UUID, never
matching by answer text. Unknown placeholders and different turns remain separate.
Writing-block structure survives the identity handoff. The smoke runner now
requires exactly one native answer and retains bounded per-gate diagnostics.

`fresh-native-reconciliation-tests-20260914-094749-669` compiled Release Kotlin
and Java and passed 48 Android tests, with zero failures/errors/skips.
`fresh-native-smoke-evidence-20260914-095115-402` passed the receipt, continuity,
cleanup and exact-readback guards. Source-size and document guards passed.
## 1720 Release And Remaining Acceptance

Release `1.1.1720`, source `72efc568e2b83d87802d43adc4817e9f0162c7a9`, is
published, remotely hash-verified and installed in place on the same Xiaomi.
APK SHA-256: `eda8284dda6e72edbd2549f5439d987939a8b001031254595b9e57f820dd63ec`.
`fresh-native-ui-readback-20260914-100321-786` confirmed the prior exact fixture
with one native answer and zero sends.

`fresh-new-native-1720-20260914-100415-999` dispatched once through the native
button. Its diagnostic query timed out before the runner could observe final
state. A subsequent read-only query confirmed one user, one answer, the accepted
native send receipt, 31 stream events, terminal history and `pending=false`.
`fresh-native-1720-readback-20260914-100702-280` confirmed the exact fixture again
with zero sends. This is first-send plus readback evidence, not a completed
first/follow-up runner or latency benchmark.

The next runner `fresh-new-native-1720-final-20260914-100734-431` was interrupted
by a different foreground package at the Java test's first assertion, before
reading the step, changing the draft or clicking Send. No further phone
navigation was attempted. The generated empty-ID handoff was preserved locally
and the exact previous verified-readback handoff restored; no message replay
or user data operation occurred.

The harness now retries only read-only state queries within the original send
deadline. The proven pre-action foreground guard is separately classified as
not sent; timeouts, missing acknowledgements and later UI errors remain unknown.
The source-contract test pins that guard before all UI actions. Foreground
changes defer restoration instead of taking over the other app.

Follow-up acceptance remains deferred. New-conversation default is still
disabled until that case passes. No temperature/power claim is made.
