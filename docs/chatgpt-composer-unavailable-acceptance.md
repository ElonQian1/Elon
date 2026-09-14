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
evidence tests are reused. Device acceptance passed below; reuse it unless a
current regression changes this scope.

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
work-mode dictation or realtime voice.

### Accepted On Adapter 411

`fresh-new-composer-unavailable-native-411-20260915-060014-013` passed on Xiaomi,
using local research 1.1.1747 built from `4633785ee`, SHA-256
`7c2d8834d15ae85c0a1faea86d6754130099137b81e00c6639f196e9ab143446`.
One production native Send, zero seed/replay, one exact user/answer and receipt;
31 private stream events, confirmed history and conversation identity. The
native composer flag was false before Send. All 77 lease samples had zero
usable composer and zero invalid-owner observations. Reply observed at 7.305s;
13.367s includes final readback, not an A/B claim. Original route, CSS and awake
state restored; no unknown write remains. The production sender's existing
default was used, not a trial arm. Normal release promotion follows this proof.

Offline regression: 57 Node tests, seven PowerShell evidence negatives and 15
Android unit tests passed. Signed research Release build passed in 370.7s.
