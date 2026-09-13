---
capability_id: android_chatgpt_private_text_input_memory_v1
implementation_status: implemented
verification_status: offline_verified
device_verification: deferred
---

# Private Text Input Readiness

September 14 source follow-on to [Fresh Text Dispatch](chatgpt-fresh-text-dispatch.md).
This removes the initial mounted-composer requirement for the existing independent
private sender when its actual official in-memory owner and draft are available.
It does not remove the WebView identity/runtime layer, invent a conversation,
change the HTTP protocol or promote additional send scopes.

## Ownership And Draft

- `chatgpt_web_committed_composer_owner.js` locates the actual committed React
  root via the observed container marker and `stateNode.current`. It follows
  bounded child/sibling membership and reuses the committed ancestor resolver.
  Only a unique, matching conversation/controller/shared-store/files-store tuple
  is accepted. Ambiguous, stale, oversized or unrecognized trees are unavailable.
- Candidate traversal is cached per document and committed root identity. Live
  props, store ownership and route are rechecked before use. The maximum is four
  roots, 8192 fibers, 512 siblings per parent and 64 composer candidates.
- `capturePrivateConversation` reads the real controller's official editor via
  the version-mapped getter/serializer. It never constructs another editor owner
  or interprets an absent DOM element as an empty draft. Only complete plain-text
  documents up to 20,000 characters are eligible.
- Draft changes use the same official editor's replacement transaction with an
  exact expected-draft comparison and no focus/scroll request. Account, document,
  route, controller and editor must still match. Preparation checks and accepted
  draft cleanup in the existing HTTP transaction use this owned reader/clearer.
  Unknown writes retain the existing barrier and are never replayed.

## Asynchronous Native Admission

`chatgpt_web_private_text_input.js` prepares a read-only binding asynchronously:
one in-flight capture, a five-second deadline and two-second failed-attempt
cooldown, driven by existing snapshots without a new polling timer. Successful
capture schedules one snapshot; timeout or a previous owner cannot publish a
late result. Cached state is validated on every use and is not saved to disk.

The adapter exposes `privateSendReady` separately from `composerReady`.
`ChatGptWebAccessPolicy.canSendText`, the native consumer input and text MCP
commands may use the former after origin, account and current-generation checks.
The page adapter is not marked DOM-ready on that evidence alone. Tools, voice,
attachment admission and Google retain their existing readiness conditions.
Content-only events cannot provide private readiness; voice snapshot continuity
copies the latest flag instead of retaining an old one.

If the mounted composer is present, its existing draft behavior remains primary.
The memory path observes the existing private-send switches and existing
new/project/temporary scope flags; active tools/uploads are not broadened here.
Unknown runtime, owner or draft means not ready, not that the website lacks the
feature. Fully loading-free cold start is not claimed.

Transaction module and instance versions are both 21. The previously mismatched
instance version could discard a settled receipt ledger on reinjection; matching
versions preserve duplicate-command protection across same-document reinjection.
Adapter version is 383.

## Evidence And Acceptance Boundary

The retained September 12 public assets are parsed, never executed or used to
export identity. `test-chatgpt-runtime-public-evidence.cjs` verifies their hashes
and AST contracts for the real controller/editor exports and committed-root
marker. The synthetic fixture composes production context, runtime and draft
modules; it does not make requests or operate on a user's conversation.

- `private-input-provider-regression-20260914-004703-855`: 379 JavaScript tests
  passed, zero failures/skips, including pinned public-source contracts.
- `private-input-final-node-20260914-005150-243`: 208 tests passed after draft
  command extraction and version alignment, zero failures/skips. Covers full
  adapter assembly, absent DOM, exact draft mutation, stale identity/route/root,
  duplicate ownership, rich/oversized input, timeout and no duplicate dispatch.
- Android production Kotlin/Java compilation passed in
  `private-text-input-native-20260914-004603-539`; the test source had an incorrect
  event accessor, corrected to the existing `Snapshot.value` contract before the
  subsequent targeted test run. This initial run is not a unit-test pass.
- `private-input-native-tests-20260914-005738-006`: 49 of 50 tests passed; the
  remaining new protocol fixture omitted the required schema/event envelope.
  The fixture was corrected without relaxing the parser. All four new tests
  then passed, zero failures/errors/skips, in
  `private-input-native-envelope-20260914-010515-939` (114 seconds). The other
  46 unchanged policy/protocol/asset tests retain their earlier passed result.

No APK packaging, publication or fresh device acceptance belongs to this batch.
The accepted existing-personal HTTP scope remains accepted; absence-of-composer
readiness is a separate pending device case. Before a later new write, resolve
the existing uncertain fresh-send fixture by read-only reconciliation; do not
resend it. Then verify one eligible production native send without a mounted
composer, one matching response, no duplicate and restoration of the prior state.
No latency, energy or temperature improvement is claimed without measurement.
