# Independent Text Dispatch: Evidence and Implementation Boundary

Date: 2026-09-12. Investigation against main `85b7bdd9a`, adapter 364.
This report does not mark independent private POST implemented or device-verified.

## Conclusion

The current sender can be evolved beyond composer callbacks. It is not blocked
by Kotlin, Android HTTP or the mere existence of React. The missing part is an
owned fresh-request transaction, not another copy of the native Send button.
The first useful target is **page-local private HTTP with native transaction
state**, keeping website identity/security in the resident WebView. Moving the
same bytes to Android HTTP is a later, separately justified transport change.
Neither stage should be described as faster or cooler before measurement.

The first candidate should cover an authenticated existing ordinary conversation,
plain text, no tools/files/temporary/project/shared context, and no active voice
or generation. Its existing parent, model and preference state must be proven
current. Unsupported contexts stay on the accepted sender before dispatch.

## What the Existing Code Does

- `chatgpt_web_text_transaction_orchestrator.js` tries the official runtime
  transaction first. That completed path already avoids a DOM Send click.
- `chatgpt_web_private_text_transaction_relay.js` is an older captured-template
  path. It refuses nonempty `openai-sentinel-*` material, rather than replaying
  one-time preparation/proof from the previous turn. Removing this rejection
  would not create a fresh-request implementation.
- Native send receipts, single-writer ownership, stream normalization and
  read-only reconciliation already exist. Reuse them instead of adding a second
  sender, transcript store or speculative success indicator.

## Observed Lower-Level Contracts

The retained September 12 public assets were parsed without executing their code:

| Asset | SHA-256 |
|---|---|
| `conversation-small-h1dtzoris1y9588z.js` | `da08c64c132306779e09ba89cac64fa560b120e7560ffdc29b3ce5a0b8ccd67e` |
| `8b34dbc2-fqgb3eqijpn96umi.js` | `1d0b132fe9b13120370395bbfbe4bf3324c4dc1213b80e591cd3c6a6db7d16a1` |

1. Conversation export `VKt` (`Lun`) obtains fresh chat requirements, turnstile
   and proof material using the website's security providers, including login
   enforcement. Composer imports it as `THe`. It is not a reusable API key.
2. Composer export `vh` (`FB`) prepares `/f/conversation/prepare`. It binds the
   selected parent, model, effort, tier, hints and conversation state, and checks
   whether a later transaction superseded its result.
3. Body builder `AB` is local, **not exported**. It serializes state including
   privacy, model overrides, messages and stream encodings. Copying an old body
   or pretending that `AB` is importable does not satisfy this dependency.
4. Local generator `VZt` consumes prepare ownership at actual request start,
   updates the stop token and coordinates fresh material on provider-controlled
   retry. It is not itself an exported independent send API.
5. Conversation export `jGt` (`DM`) owns the SSE request, deadlines, credentials,
   request/response integrity and terminal event. Export `MGt` adds retry logic.
   Calling bare fetch with only the earlier proof tuple is not equivalent.
6. The generator can hand a stream to another transport, then resume or poll.
   Root SSE closure alone is not proof of completion. The current native stream
   observer cannot simply be assumed to cover every private dispatch variant.

The public AST contract test pins these boundaries. Passing it proves only the
reviewed source shape, not account eligibility, live request success or latency.
A same-turn unauthenticated refresh of the three known CDN assets failed before
receiving HTTP responses, so this report uses retained, hashed evidence rather
than claiming a fresh website deployment audit. Wireless ADB and read-only APK
MCP were reachable and reported adapter 364; no message was sent in this audit.

## Implementation Order and Acceptance

1. **Fresh transaction context.** Bind account/workspace, document, conversation,
   selected parent, native request/user-message IDs, model/effort/tier and privacy.
   Native state owns the command; only identity material stays in the page.
   Do not add a global composer-ready gate to unrelated reads.
2. **Fresh preparation and one request.** Resolve the observed security and
   preparation contract for this exact command, then serialize the supported
   plain-text body from current state. Keep all login/account protections and
   response-integrity handling. No captured proof/header replay or forced flags.
3. **Native stream and reconciliation.** Connect the existing decoder, ledger,
   cancellation and receipts. Recognize authoritative completion and any handoff;
   retain native text while refreshing canonical history. A changed page cannot
   attach the old reply to the new conversation. Unknown writes reconcile
   read-only, never silently retry through the composer.
4. **One bounded production trial.** Use an owned synthetic existing conversation.
   Send once, observe a fresh private dispatch receipt and matching native stream,
   then send a follow-up and exercise explicit stop. Verify exact ownership,
   no duplicate turns and restoration. At least one missing-composer case must
   demonstrate reduced UI dependency. This is a proposal, not a passed test.

Only after that narrow path is genuinely usable should it become the default
for eligible commands. Then expand new-chat allocation, tools, attachments,
temporary/project contexts and other account modes. Android HTTP ownership can
be evaluated after page-local private dispatch is correct; it is not a shortcut
around fresh preparation or stream reconciliation.

## Delivery Status

This batch adds a reproducible public-source contract test and this audit. It
does not change the production transport, enable an unverified sender, export
credentials, clear app data, record audio or require another APK release.
The new public AST suite plus existing private-transaction, transaction-lifecycle
and runtime-submit suites passed **154 tests, zero failures and zero skips**.
Public source was parsed, never imported/executed. Logged run:
`text-dispatch-boundary-audit-20260912-173301-163`.
Independent dispatch remains a real implementation/acceptance gap, not merely
a configuration switch. Reuse [the accepted coordinator](../chatgpt-same-origin-text-transaction.md)
and [runtime sender](../chatgpt-official-runtime-text-submit.md) while closing it.
