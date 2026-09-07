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
- One pending owner is bound to document token, document, URL, message count,
  message revision and composer draft.
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
  A verified current/previous-build modal can offer the native confirmation below;
  an unknown modal remains on the official path without automatic approval.

## Native guest confirmation

Runtime v2 reuses the shared committed React-owner resolver introduced in
`4e03abcea`; the text sender retains its existing behavior. The modal helper
accepts only the two source-evidenced guest components and exact action/close
callbacks from the current runtime profile. It verifies logged-out/no-session
state, the connected committed subtree, unique action and current modal identity.
It calls the website's own router/close callbacks, not a DOM click or guessed POST.

- A page-local random ticket binds one explicit confirm/cancel decision to the
  original document and chat context for at most 60 seconds. No credentials or
  conversation content enter the receipt. Approval recaptures and compares
  context and rejects an edited draft; cancellation can preserve an edited draft.
- Native UI restores the existing transcript before presenting `New chat?`,
  with clear-and-create versus keep-current actions. The native lease lasts at
  most 50 seconds; leaving the foreground or dismissing cancels, never approves.
  Stable controls are `web_chat_new_conversation_confirm` and
  `web_chat_new_conversation_cancel`. Protocol tags/tickets are not user text.
- A decision consumes the page lease before invoking the official callback.
  Unknown/expired/repeated decisions cannot replay navigation or use DOM fallback.
  A v1 runtime must reject decisions rather than treat them as fresh new-chat clicks.
- The official handler records `HasSeenNewChatModal` when opening its modal,
  before consent. After expiry/suspension, and before/after an asynchronous import,
  an existing modal is therefore rebound instead of invoking `newChat` again.
  Missing runtime discovery must not bypass that existing modal.
- Streaming/context state is reset only after confirmed empty-chat settlement,
  not on the first click or when confirmation is pending. Native recovery now
  requests a snapshot only; it cannot force a WebView reload during this operation.

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

APK `1.1.1551` contains navigation v1, text-runtime v10 and background discovery
backoff. Publication and replacement installation were verified separately from
acceptance; the handset was locked afterward. The snapshot-only recovery repair
`62a5a7708` and native guest-confirmation batch are not in that installed artifact.
See [1551 delivery and recovery evidence](reports/chatgpt-runtime-release-1551.md).

The native-confirmation production/test compilation passed in 302.7 seconds,
with 26 selected JUnit tests and no failures/errors/skips. Its log stem is
`new-chat-native-confirmation-android-20260908-20260908-042754-927`.
The continuation preserves exactly the same Kotlin content (only checkout line
endings differ), so this compilation is reused rather than restarted.

Six new red cases reproduced lease-expiry/suspension reentry, a preexisting modal
with missing bindings/helper, a modal opening during import, and v1 decision
misdispatch. The corrected navigation, production wiring, text/current consumers,
runtime assets and Windows guard suites pass 185 Node runner cases, zero failures:
`new-chat-confirmation-reentry-red-20260908-20260908-044752-779` and
`new-chat-confirmation-reentry-green-20260908-20260908-044947-777`.

These are offline evidence, not an observed guest-data deletion or live protocol
acceptance. Group the source changes into the next APK. Registered action/modal
availability, rendered native consent, actual empty-chat settlement and the direct
send route still need production acceptance. The overall Goal remains active.
