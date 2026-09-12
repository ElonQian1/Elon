---
capability_id: android_chatgpt_fresh_text_dispatch_v1
implementation_status: partial_source_candidate
verification_status: offline_contract_and_transaction_tests
production_default: false
---

# Fresh Text Dispatch

September 12 source implementation following the
[protocol audit](reports/chatgpt-independent-text-dispatch-audit-20260912.md).
This is not an accepted production transport or an Android HTTP implementation.
Reuse these modules for subsequent work rather than repeating the audit or
creating another native sender. The accepted runtime sender remains the default.

## Implemented

- `chatgpt_web_fresh_text_context.js` binds a personal authenticated account,
  document, existing ordinary conversation, selected completed assistant parent,
  model, effort, service tier and privacy. Initial ownership still comes from the
  committed composer host; subsequent checks read captured stores and the live
  conversation registry, not the Send callback or composer-ready flag.
- `chatgpt_web_fresh_text_request.js` builds a fresh supported body and message
  IDs. Each command issues its own preparation request and obtains current
  provider security material. No captured proof, cookie export, old conduit or
  prior message body is replayed.
- `chatgpt_web_fresh_text_transaction.js` calls the version-pinned lower HTTP/SSE
  transport, not `submitComposer` or the retrying generator. It reuses the native
  stream observer and existing command router. One command owns one dispatch;
  an uncertain write keeps its barrier rather than falling through to another
  sender. Preparation, response headers, stream and reconciliation have separate
  bounded deadlines. Late completions cannot re-enter the send path.
- `chatgpt_web_fresh_text_reconcile.js` uses the official history fetch-and-apply
  helper after stream completion. Only a matching user ID, parent and completed
  selected branch may update the website tree. The account, document and branch
  are checked again immediately before apply. No reload or empty native snapshot
  is used to synchronize state.

The binding additions apply only to the observed `web_20260912` asset profile.
Unsupported contexts can use the accepted sender only before preparation and
after claiming the same command once. Active writers, auth enforcement after
capture, timeouts and ambiguous delivery cannot authorize an automatic replay.
There is no periodic polling or second transcript store in these modules.

## Not Completed

- Live fresh preparation, first send, follow-up and native stream acceptance.
  Offline source parsing and fixtures are not server or device evidence.
- Server-confirmed stop for this new request owner. Cancelling its reader does
  **not** claim server generation stopped; the router returns unknown and retains
  ownership. Implement the owned stop contract before consumer activation.
- Recovery for delayed history and stream-handoff variants. A history response
  that cannot prove completion leaves a barrier, not a fabricated success.
- Initial ownership without a mounted composer. This version removes the
  submission callback/readiness dependency but does not claim zero DOM bootstrap.
- New chats, tools, attachments, temporary/project/shared chats, non-personal
  workspaces and unobserved website asset profiles remain on existing paths.

The candidate requires both the existing transaction flag and the page-local
`__elonChatGptFreshTextDispatchEnabled === true`. No production host or consumer
setting currently enables the latter. Do not enable it as a default merely
because the source tests pass. A controlled trial switch must remain tied to the
owned transaction and the exact reviewed provider profile.

## Verification

Focused suites cover fresh bodies/security order, one-write ownership, duplicate
commands, account/branch/draft changes, late timeout results, stopped readers,
authoritative history guards, asset assembly and existing sender compatibility.
The pinned public-source AST suite verifies the lower request, security, stream
and history exports without importing or executing downloaded website code.
The final focused run passed **279 tests, zero failures and zero skips**:
`fresh-text-receipt-final-20260912-193347-443`. This includes the existing private
stream transport and stream-resume regressions, not just candidate fixtures.
No APK build, publication, phone message or audio test is part of this source
batch; existing installed behavior remains unchanged.
