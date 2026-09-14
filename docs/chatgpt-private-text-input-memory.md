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
- Candidate traversal rechecks the current committed children on each lookup;
  root-object reuse cannot retain a negative or obsolete owner result. Live
  props, store ownership and route are rechecked before use. The maximum is four
  roots, 8192 fibers, 512 siblings per parent and 64 composer candidates. This
  reuses the [1737 ownership repair](reports/chatgpt-tool-owner-20260914.md).
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
one current-owner capture, a five-second deadline and two-second failed-attempt
cooldown, driven by existing snapshots without a new polling timer. Successful
capture schedules one snapshot; timeout or a previous owner cannot publish a
late result. Identity, document or scope changes retire the old observer and
its deadline immediately, without cancelling shared runtime loads. The next
owner need not wait for the old capture; late callbacks cannot retire its wait
or publish an old draft. Cached state is validated on every use and is not saved
to disk.

The adapter exposes `privateSendReady` separately from `composerReady`.
`ChatGptWebAccessPolicy.canSendText`, the native consumer input and text MCP
commands may use the former after origin, account and current-generation checks.
The page adapter is not marked DOM-ready on that evidence alone. Tools, voice,
attachment admission and Google retain their existing readiness conditions.
Content-only events cannot provide private readiness; voice snapshot continuity
copies the latest flag instead of retaining an old one.

If the mounted composer is present, its existing draft behavior remains primary.
The memory path observes the existing private-send switches and existing
new/project/temporary scope flags. Its tool scope now matches the sender:
personal existing-chat Search uses the accepted default unless explicitly
disabled; other supported tool combinations require the explicit tool switch.
The same context validates current account, model, tool eligibility and draft.
Already-associated attachments follow the exact explicit attachment switch too;
readiness reuses their current owned upload lease and does not consume files.
This does not enable uploads, new composer-free sends or additional tool defaults.
Unknown runtime, owner or draft means not ready, not that the website lacks the
feature. Fully loading-free cold start is not claimed.

At the initial checkpoint, transaction module and instance versions were both 21. The previously mismatched
instance version could discard a settled receipt ledger on reinjection; matching
versions preserve duplicate-command protection across same-document reinjection.
That checkpoint used adapter 383; the current input module is 3, adapter 402.

## Tool Scope And Pending-Owner Repair

The September 14 follow-up changes only the existing input-readiness module and
adapter activation constant; no new sender or private endpoint is introduced.
Previously input capture omitted `allowPersonalSearch` and `allowTools`, even
when the independent sender could accept that state. A pending capture also
blocked a new identity/scope for up to its five-second deadline.

`private-input-tool-red-20260914-230815-020` reproduced four failed cases;
`private-input-owner-red-20260914-231100-727` reproduced four owner-wait failures.
After the repair, `private-input-scope-final-20260914-231136-888` passed 280 tests,
zero failures/skips, including production context/runtime/draft composition,
default scope, tool checks, sender transactions and complete adapter assembly.
Entitlement and HTTP fixtures are synthetic, not provider acceptance.

This follow-up is source-only for the next grouped Android build. USB reported
no device; the bounded wireless connection timed out and mDNS found no service.
No APK was replaced, no message was sent and no device pass is claimed. Existing
Search HTTP acceptance is retained; its absent-composer UI boundary still needs
one controlled production check after the device returns.

## Evidence And Acceptance Boundary

The attachment follow-up fixes the omitted `allowAttachments` input scope,
without changing attachment upload admission or independent-send defaults.
It covers a missing text composer with an already-owned, mounted file-store
owner, not fully DOM-free uploading. Switch changes invalidate cached readiness;
account, model, file state, metadata and lease checks remain authoritative.
`private-input-attachment-red-20260914-233530-421` reproduced four failures.
The new fixture composes the production attachment composer/lease, fresh context,
memory draft and input modules; it checks TXT/PDF/PNG readiness, no request or
file consumption during editing, revoked ownership and exact sender/input scope
parity. `private-input-grouped-regression-20260914-233724-894` passed 338 tests,
zero failures/skips, and includes the prior
tool/owner fix and the existing sender, attachment, default-scope and assembly
regressions. These are synthetic offline checks, not real attachment HTTP
acceptance. The next grouped release carries both input fixes; device checks
remain deferred while USB and wireless ADB are unavailable.

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
