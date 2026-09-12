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
- `chatgpt_web_fresh_text_stop.js` adds owned server stop for this candidate. It
  reads the original command's server branch, uses its own fresh preparation
  conduit and turn trace once, and submits the observed `/stop_conversation`
  contract through the existing identity client. It does not invoke a generic
  DOM Stop control or stop another request through the page's current callback.

The binding additions apply only to the observed `web_20260912` asset profile.
Unsupported contexts can use the accepted sender only before preparation and
after claiming the same command once. Active writers, auth enforcement after
capture, timeouts and ambiguous delivery cannot authorize an automatic replay.
There is no periodic polling or second transcript store in these modules.

## Not Completed

- Live fresh preparation, first send, follow-up and native stream acceptance.
  Offline source parsing and fixtures are not server or device evidence.
- Live server-stop and follow-up acceptance for this new request owner. Source
  integration is present, but no real stop response/history was observed in this
  batch. Cancelling a reader alone still **does not** confirm server stop.
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

## Owned Stop Source Batch

September 13: `android_chatgpt_fresh_text_stop_v1`, source-integrated, not
device-verified. Reuse this implementation rather than adding another stop owner.
The reviewed September 12 conversation `KM`/`xWt` export uses the provider's
`3922476776` and `877631007` gates, source message `chime_version`,
`conversation_id`, `exclude_async_types`, `x-conduit-token` and
`x-oai-turn-trace-id`. The existing shared `$3` binding is the same exported `b5`
gate reader. Composer `VZt` retains the consumed preparation conduit as the stop
conduit. These source facts are pinned in the public AST evidence test; no
downloaded website code is executed.

- Before dispatch, stop cancels preparation so late security/preparation results
  cannot send. After dispatch, it checks account/document/branch ownership and
  reads history without applying it before sending one stop request.
- Concurrent taps join one operation. After an attempted stop with uncertain
  delivery, another explicit tap can only verify history, not repeat the POST.
  Stop has a 12-second deadline and at most three terminal-history checks.
- Only matching terminal history plus website-store reconciliation releases the
  writer. Complete or partial assistant output requires server async status null
  or the reviewed UNREAD=4; EOF and HTTP acknowledgement alone are insufficient.
  A user-only stopped turn additionally requires a successful stop response.
- Native partial text remains visible; stream finalization happens only after
  this confirmation. Partial assistant turns can become the next private parent.
  A stopped user-only turn still uses the accepted sender for its follow-up,
  because fresh-dispatch parent capture remains assistant-only.
- No new Android HTTP identity channel, new-chat/tool/attachment expansion or
  production-default change is implied. Adapter 368 assembles the new module;
  transaction v2 upgrades only when the older owner has no pending write.

This source batch follows the grouped workflow: focused protocol/lifecycle tests
now, Android compilation, APK packaging and production UI acceptance later with
the other pending source capabilities. No stop or message was sent on a phone.

The September 13 stop batch passed **279 tests, zero failures and zero skips** in
`fresh-stop-final-20260913-010356-876`. This run includes pinned public-source
checks, fresh transaction/stop integration, pre-dispatch cancellation, partial and
user-only terminal cases, lost stop responses, identity/branch changes, and the
existing runtime stop, attachment submit and private-stream regression suites.
The final context check permits our server-owned streaming turn to be inspected,
while refusing another live runtime writer and refusing to reconcile as stopped
until the local provider state is idle or unread.
It is distinct from the earlier September 12 sender run with the same total.
