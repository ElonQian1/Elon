# Registered website new-conversation action

Capability: `android_chatgpt_runtime_new_conversation_v1`.
Implementation: implemented. Offline verification: passed. Device acceptance:
pending. This is page-owned runtime navigation, not an independent private HTTP
reset or permission to discard guest history without confirmation.

## Evidence and integration

APK 1550 could send through the existing fallback but its guest-root new-chat
request returned no entry found. The current public guest header has an icon
anchor with tooltip-only text; the previous label scan may miss it. The recorded
device failure alone does not prove which particular DOM selector failed.

Both public source builds in [runtime bindings](chatgpt-private-runtime-bindings.md)
expose the same registered-action API: shared `Ur` returns action configuration
and `zr` invokes the latest live registration. Their local names changed from
`d2`/`Vzt` to `p2`/`iBt`. Composer `TUt` registers `newChat` with the label ID
`keyboardActions.newChat`, global scope and the official shortcut. Current `Kpr`
(previous `bfr`) installs the callback that handles guest confirmation and then
uses the conversation router, preserving the website's state ownership.

Shared `zr` validates the current disabled state before invoking the action.
Its true return value means invocation, not readiness. It does not prove that
a navigation succeeded or bypassed the guest-discard modal.

`chatgpt_web_private_new_conversation.js` consumes only those two verified
aliases through the existing versioned runtime bindings. The production
conversation command delegates to it before scanning a visible DOM button.
No arbitrary action API, credentials, forced reload or second sender is added.

## Lifecycle

- Ordinary root and saved-conversation routes are supported. Project, temporary,
  query/hash routes and unknown builds retain their existing path.
- One pending owner is bound to document token, document, URL and message count.
  Repeated requests remain busy even if navigation temporarily hides the editor.
- Cached runtime exports are used synchronously; missing exports use the shared
  bounded import owner. Missing discovery is unknown, not missing capability.
- Before invocation, unavailable/disabled/unrecognized registration can use the
  existing path once, only while the original context is unchanged.
- Once invoked, false/unknown/exception/timeout never triggers a second DOM
  attempt. Document/route changes terminate observation without replay.
- Success requires the ordinary root route, ready composer and zero messages
  continuously for 160 ms. Empty content on the old saved route is not success.
  Observation lasts at most five seconds, uses bounded 80 ms timers, then stops.
- Exact official `modal-no-auth-new-chat` detection returns confirmation-required.
  The native result keeps that distinction; no destructive confirmation is clicked.
  Native guest-confirmation UX remains separate from this navigation transport.

## Verification and remaining work

The initial 26-case run passed 24 and failed both production wiring checks.
Four additional red cases exposed readiness-loss duplicate dispatch, false
success at the old saved route, unaccepted-mode settlement and a synchronous
discovery exception. All were corrected before integration verification.

The combined new/old navigation, runtime bindings/assets, current consumers,
text, attachment, stop, regeneration and receipt suites passed 277 Node runner
cases, with zero failures/cancelled/skipped cases. Log stems:
`new-chat-runtime-red-20260908-20260908-031644-662`,
`new-chat-runtime-guards-red-20260908-20260908-031825-011`, and
`new-chat-runtime-green-20260908-20260908-032007-619`.

The new module is not inside APK 1550. Group it with the submitted deep-composer
and background-catalog retry fixes for one build/install. Real registered-action
availability, guest confirmation, actual blank-route settlement and the direct
send route still need production acceptance. Keep the overall Goal active.
