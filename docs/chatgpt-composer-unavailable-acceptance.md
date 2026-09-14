# Composer-Unavailable First Send

Capability: `android_chatgpt_fresh_new_conversation_text_dispatch_v1`.
This is an acceptance extension of the existing sender, not another transport.

## Scope

`smoke-chatgpt-fresh-text-dispatch.ps1 -NewConversation -FirstOnly -UseDefault
-ComposerUnavailable` opens one empty personal conversation and adds a temporary
CSS lease through that APK's locally forwarded research WebView. It hides every
selector used by the real adapter's `findComposer`; it does not replace the
locator, editor/controller, draft store, network response or send function.

The production native composer and Send button remain visible and are used by
the existing semantic runner. Before the click, MCP must report
`private_send_ready=true` and the owned CSS lease must prove no usable official
composer. The old native transient-menu policy may retain `composer_ready=true`;
record that value separately, never use it as physical DOM evidence. The ordinary independent
send receipt, one matching user/answer, stream completion, exact conversation
identity and history reconciliation are all still required. CSS samples must
have zero visible-composer or invalid-owner observations through final readback.

This demonstrates independence from **usable official composer DOM**, not a cold
page with all editor/runtime modules physically absent. React retains its real
memory editor; do not claim a DOM-unmount or cold-bootstrap acceptance from this.

## Safety

- Only `127.0.0.1` on the selected APK process's ADB-forwarded DevTools socket.
- Requires the research build flag, exact adapter version, document identity,
  unique target, empty personal route and the smoke's existing idle guards.
- No Cookie export, microphone, private text dump, request interception or replay.
- Per-run ownership nonce protects another runner's lease. The style is removed
  in `finally`, or automatically after four minutes if the runner disconnects.
- Cleanup removes only this local forward. Existing uncertain-write navigation
  guards and the external controlled-message handoff remain in force.
- A research APK is local-only. Restore the same signed production APK and verify
  its hash, login, draft and pending-write state; never publish the debug flag.
- `diagnose` is a bounded read-only capture of the existing memory owner. It emits
  only readiness booleans and allowlisted failure codes, never identity or text.

## Evidence

The lease's offline tests cover CSS restoration, timeout, route continuity,
document replacement, lost style, wrong owner, production-build rejection,
origin/endpoint restrictions and selector parity. The existing native send
evidence tests are reused. Device acceptance is pending; record its result here
before broadening the capability's verified scope.

### Native Admission Regression

On research release 1747 / adapter 410, the CSS lease exposed a real native
admission defect: no recording was active, but three unlabeled round controls
plus absent composer DOM produced `dictation_active=true`. The native Send
button disappeared. The independent memory owner still captured successfully;
zero send receipts, attempts or messages were observed. The exact owned unsent
draft was removed, the previous accepted fixture and original view restored,
and the failed attempt archived as not dispatched, with no replay permission.

Adapter 411 requires actual capture evidence for structural dictation controls;
explicitly labeled official dictation controls remain supported. Native transient
readiness retains model/capability metadata without relabeling a verified private
editor as a ready DOM composer. Policy/integration tests cover both real recording
and the idle no-composer layout. This fix does not change private dictation,
work-mode dictation or realtime voice. Device revalidation remains required.
