# ChatGPT Page Command Delivery Recovery

Status: published and installed in APK 1.1.1698. Cold and background-return
read-only commands passed on the production native surface. Forced missing-bridge
repair is offline verified, not device exercised. This is a shared native-to-page
delivery boundary, not another private sender or a new provider protocol.

## Observed Failure And Source Gap

The September 13 read-only check on APK 1697 could display a cached conversation
while native state reported `bridge_state=ready`, `composer_ready=false` and page
generation 3. A `document_state` diagnostic timed out without a page receipt.
It sent no message and did not change login, app data or VPN settings. Cached
content and a native ready flag do not prove the page command bridge is alive.

Source inspection found `onHostPaused` disposes/deletes the page bridge, while
the old `runCommand` expression silently did nothing when that bridge was absent.
The native document/ready state could outlive it. This is a reproducible source
gap; the live diagnostic did not directly inspect the missing object and does
not prove it caused every previous navigation or seed failure.

## Delivery Contract

- The current document token and exact page URL bind each queued command.
  The page checks both atomically before invoking its command handler.
- A callable bridge is entered immediately, once. There is no extra network
  request, composer check, preflight polling, reload or provider-success receipt.
- Only `missing`, produced before entering the handler, permits repair.
  Concurrent missing commands share one repair using the existing adapter asset.
  It preserves the document token, page and existing identity layer.
- Each proven-unsent command is attempted at most once after repair. At most
  32 delivery records remain pending, each for five seconds; repair is also
  bounded. Expired waiters cannot start queued injection or late dispatch.
- Handler/evaluator exceptions, lost callbacks and ambiguous results never
  authorize replay or a fabricated known-unsent failure. Existing operation
  ownership and reconciliation remain authoritative after entry.
- Page replacement, pause and route drift invalidate pending delivery. Old
  failures do not restore the previous conversation over a new selection.
- Known pre-entry failure can return an explicit unsent error to an existing
  command waiter. Snapshot/skin maintenance does not produce user error toasts.
- Observations contain reason codes only. Successful warm commands are silent;
  recovered delivery is distinguished from successful provider execution.

This does not remove the WebView identity/runtime layer, duplicate any private
transport, change the proxy, or add an Android HTTP credential path. Writing
Block save, directory reads and text commands reuse this same boundary.

## Verification And Next Acceptance

The actual JavaScript invocation asset covers warm/missing bridges, route and
token drift, throwing accessors and throws after write entry. Kotlin tests cover
single-flight repair, timeouts, duplicate/late callbacks, pause, context changes,
queue bounds, evaluator exceptions and silent warm operation.

`chatgpt-command-delivery-js-final-20260913-092229-386` passed 82 cases with no
failures or skips, including the existing fresh-text and Writing Block save
contracts. Release Kotlin/Java compilation and all 36 targeted Android tests
(six suites, no failures/errors/skips) passed in
`chatgpt-command-delivery-android-20260913-092250-727` in 289.1 seconds. The new
delivery controller has 15 of those tests; the remaining suites cover handshake
and existing native Writing Block/code-block behavior. Source-size and document
modularity checks passed. No new device pass is implied by these offline tests.

This source batch and the shared request-sequence correction shipped together
in 1698. Cold/return receipts took 152/685 ms including MCP automation, preserving
the conversation, draft and page generation. The stop/follow-up test still failed
before sending: reopening the unchanged current route filtered out the full
snapshot needed to restore native readiness. See the
[1698 acceptance and same-route correction](reports/chatgpt-command-recovery-1698.md).
Do not repeat accepted Writing Block editing/export/save or ordinary fresh-text
send scopes without regression.

Implementation: `ChatGptWebCommandDelivery.kt`, `chatgpt_web_command_delivery.js`
and `ChatGptWebPageAdapter.kt`. Tests:
`ChatGptWebCommandDeliveryTest.kt` and
`scripts/test-chatgpt-web-command-delivery.cjs`.
