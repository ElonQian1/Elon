---
capability_id: android_chatgpt_fresh_text_dispatch_v1
implementation_status: completed
verification_status: production_ui_verified
production_default: true
scope: authenticated_existing_personal_plain_text
---

# Fresh Text Dispatch

September 12 source implementation following the
[protocol audit](reports/chatgpt-independent-text-dispatch-audit-20260912.md).
The existing ordinary personal-conversation scope is now `completed`, after
production native-button first-send, follow-up, stream and stop acceptance on
APK 1.1.1695. Transaction v7 enables this scope by default. It is a page-owned
private HTTP implementation, not Android HTTP and not removal of the WebView
identity/runtime layer. Reuse these modules; do not repeat the completed audit.
Other contexts retain the established sender before any independent dispatch.

The September 13 [Search/Create Image](chatgpt-fresh-tool-text-dispatch.md) and
[owned project](chatgpt-fresh-project-text-dispatch.md) extensions are implemented
and offline-tested, but not yet released/device-accepted or default-enabled.
Reuse these batches; the completed scope remains existing personal plain text.

## Accepted Scope And Promotion

- `fresh-text-native-1695-20260913-070727-877`: two native send clicks, no new
  seed, two unique user turns, both matching replies and both terminal-history
  reconciliations. The handoff delivered 20 events for the follow-up, including
  input, delta, assistant completion and metadata. No pending writer remained.
- `fresh-text-stop-ui-1695-20260913-071502-019`: native stop button clicked for
  one fresh long-response turn; stop/history confirmation completed and released
  the writer. The prior flag-gated run `fresh-text-stop-native-1695-20260913-071025-899`
  timed out and is not counted as a successful stop test.
- All three successful cases restored the original conversation/draft and
  screen-awake setting. Cookie, app data, voice and proxy settings were preserved.
- The UI harness observed the first/follow-up replies at 10.3/11.0 seconds; these
  include focus, semantic automation and MCP polling. They are **not** network
  TTFT or proof of reduced latency/temperature. The verified benefit is complete
  independent request/delivery ownership without invoking the website Submit
  callback, with deterministic reconciliation and no duplicate sends.
- Adapter 374 projects the exact current independent writer into generation
  state even before the first text. Another conversation cannot inherit it.
  Stream transport v21 also rejects a late passive status response while owned
  delivery is active. These promotion changes passed 163 focused tests with no
  skips (`fresh-text-default-verified-20260913-072002-847`).

The page-local `__elonChatGptFreshTextDispatchEnabled = false` remains an explicit
kill switch; disabling the existing private transaction flag also disables this
path. An explicit trial can override only the former for one owned command.
### Default Release

- APK **1.1.1696** / code **1696**, source
  `199a3af746b87f4e749223b4f8d6cea6f44eab7b`, is published and installed on the
  existing Xiaomi with an in-place update. Published metadata was independently
  checked; APK SHA-256:
  `406b258451dabc70c450e26f915221222ed73b4a664bbe9df86a621eafac99af`.
- Release build, Android release checks and publication passed in
  `fresh-text-default-release-20260913-072906-426`. The optional automatic
  worktree-cleanup warning did not affect publication or verified installation;
  the required task finish remains a separate operation.
- `fresh-text-default-native-1696-20260913-073724-222` passed with
  `-UseDefault -FirstOnly`: no experimental permit, one native send click, no
  new seed, one unique user turn, matching reply, 21 delivery events and confirmed
  terminal history. The owner completed with `pending=false`; original
  conversation/draft and screen-awake setting were restored.
- The observed reply was at 9.2 seconds and the whole send case at 14.9 seconds,
  including semantic UI automation and MCP polling. This verifies default
  routing, not a network-TTFT or thermal improvement claim. The accepted first,
  follow-up and stop cases above were not expanded or repeated unnecessarily.

## Controlled Production Acceptance

The existing MCP protocol command now supports `fresh_text_trial_start`,
`fresh_text_trial_state` and `fresh_text_trial_end`. Start permits exactly one
fresh-dispatch attempt within 120 seconds, bound to the current document,
account and route. It sends nothing by itself, does not modify the default flag,
and does not extend the deadline on repeated starts. End removes only unused
permission; it cannot cancel or release an unresolved write owner.

The `elon.fresh_text_trial.v1` receipt distinguishes this owner from the older
`private_text_v1` relay by attempt, dispatch, acceptance and reconciliation
evidence. It exposes no message, account identifier, request body or credentials.
The Android command receiver validates the same bounded schema.

`scripts/smoke-chatgpt-fresh-text-dispatch.ps1` clicks the production native send
button with a marked synthetic prompt, checks a unique user turn and native
reply, then checks owner reconciliation before a follow-up. It reuses its owned
fixture on subsequent runs and restores the original conversation/draft and
screen-awake setting. This acceptance control is not a completion/default-enable
claim; live results belong below after a real run.

### September 13 Device Result

APK **1.1.1693**, source `c1cd275e84a9ac988d4248f26f46d3332749798d`, was
installed without resetting app data. The native composer is initially collapsed;
the acceptance runner now reuses the existing semantic focus step before typing.
It matches this command's response rather than mistaking a later status row for
the assistant, and retains its marked fixture as soon as its route is established.

In `fresh-text-native-1693-20260913-060440-671`, the accepted sender's seed passed
in 10.7 seconds. Exactly one candidate click reached the fresh owner: one dispatch,
successful SSE response headers, but no matched native answer and no confirmed
history reconciliation within 90 seconds. The original conversation and awake
setting were restored. This is **not** successful fresh-send acceptance. No
follow-up or active stop was attempted, and the normal sender remains default.

Transaction v5 adds bounded event-type counts and history rejection codes to the
trial receipt. These distinguish protocol handoff, missing/foreign history and
store reconciliation without exposing messages, IDs, topics or credentials. The
reviewed website `DM` yields `{event,data}` packets; its composer additionally
handles `stream_handoff`.

APK **1.1.1694**, source `827fbf7e7`, established the missing delivery boundary in
`fresh-text-native-1694-20260913-063510-307`: one candidate click, successful
response headers, exactly three events (`delta_encoding`,
`resume_conversation_token`, `stream_handoff`) and matching history still in
`server_active`. No answer was received within the 45-second observation window.
No follow-up or stop was attempted. Returning to the original route does not
mean the unresolved independent writer was released.

### In-Band Stream Handoff

The reviewed September 12 composer `ZZt -> HZt` subscribes an offered
`subscribe_ws_topic` through shared export `ej -> G9e -> getTopic`. Its topic
items use `conversation-turn-stream`, `stream-item`, `encoded_item`, linked
stream item IDs and a separate authoritative `done`. The public AST evidence
test checks these exact contracts without executing the downloaded website.

- `chatgpt_web_fresh_text_stream.js` follows only the offered topic, with history
  catchup, duplicate suppression, missing-parent rejection, bounded queues and
  an idle timeout. It registers listeners before subscribing, disposes them on
  completion/cancellation/error, and never acquires an already-used provider
  topic. The provider client is reused; no credentials or endpoint are guessed.
- `chatgpt_web_private_owned_stream.js` feeds the existing native session and
  v1 delta decoder. There is no second transcript cache. It validates the exact
  conversation/document owner and suppresses passive duplicate observations
  while the independent sender owns delivery.
- Transaction v6 connects these modules; bindings v21 exposes the topic helper
  only for the reviewed profile. Private-stream transport v20 preserves the
  accepted passive path outside owned delivery. Root SSE EOF or an encoded
  `[DONE]` inside a topic is not treated as topic completion. Terminal history
  reconciliation is still required before the next send is allowed.
- Unknown handoff, stream gaps, server errors and lost ownership stop local
  consumption and retain the uncertain-write barrier. They do not replay the
  user message or invoke a second sender.

The targeted source/integration/public-contract run passed **159 tests, zero
failures and zero skips** (`fresh-text-handoff-verified-20260913-065528-775`).
Those tests preceded the device acceptance and promotion above; they were not
used alone to enable the default.

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

## Lifecycle Follow-on Source Batch

September 13 follow-on source batch: the experimental 32-send lifetime ceiling
is replaced by 32 retained receipts and an exact retired-command sequence range.
The sequence comes from `ChatGptWebObservedState.beginCommand` (positive base-36
Long IDs). Only settled/reconciled receipts can retire; uncertain writes retain
their owner. Old commands outside the retained window are rejected, including
when the private sender is disabled, rather than replayed through another sender.
Diagnostics v6 count up to 65535 without limiting sends; the receiver also accepts
bounded v5 receipts from an older active writer. The 92 focused JavaScript cases
passed in `fresh-text-receipt-lifetime-20260913-075142-300`. This is not a repeated
live-send stress test.

Context v2 and transaction v9 allow a user-only parent only after this exact
sender received server stop acknowledgement and reconciled the same stopped
branch. A bounded one-parent proof rechecks account, document, registry, parent
and server-id ownership; it survives pre-dispatch unavailability but is removed
on the next dispatch or document replacement. Partial-assistant follow-ups keep
their real assistant parent. Reader cancellation alone grants no continuation.
Stop completion can retain this proof even when the settled writer was already
released; it cannot grant it to another active writer. Diagnostics v6 expose only
`parent_role` so live user-only and partial-assistant results remain distinct.

The combined 158 JavaScript cases passed in
`fresh-text-continuation-20260913-075939-380`, including actual transaction,
stop/history modules, both parent variants, a pre-dispatch interruption and no
second stop request. Release Kotlin/Java checks and all six targeted Android
receipt tests passed in `fresh-text-lifecycle-android-20260913-080014-059`.
`-StopThenFollowup` reuses native send/stop buttons and records the observed
parent role rather than assuming the server stopped before first text.

### Lifecycle Release And Acceptance Boundary

- APK **1.1.1697** / code **1697**, source
  `ec8e8448a15a658aff4a7fe17357fa5a7a0e8305`, was published and installed in place
  on the existing Xiaomi. Metadata and installed version were independently
  checked. APK SHA-256:
  `367e7c38ac6c52ba1f3aeb4e7d30875ec5f736ee79857b8571fc0a6db2320c72`.
- `fresh-text-lifecycle-release-20260913-080628-430` passed in 489.7 seconds.
  The optional worktree-cleanup warning did not prevent publication or verified
  installation; the required task finish is still separate.
- `fresh-text-stop-followup-native-1697-20260913-081521-650` **did not pass**.
  The single seed click received no matching reply within 90 seconds. It stopped
  at `stage=seed`, with `candidate_clicks=0`; neither the new stop/follow-up
  behavior nor its parent role was exercised. The original conversation/draft
  and awake setting were restored. The seed was not replayed.
- A later read-only attempt to locate/reuse an earlier marked fixture also
  timed out during navigation. Observed state became `bridge=connecting`,
  `page_generation=3`, with timed-out navigation receipts. The final UI was
  returned to `conversation_home`; this does not prove the diagnostic navigation
  completed its original-route restoration.
- One in-APK network check returned ChatGPT HTTP 200 in 635 ms and Google HTTP
  204 in 276 ms. This rules out calling the observation a total network outage,
  but does not prove private endpoints or the WebView runtime were healthy.
  No root cause or lifecycle regression is established by this incomplete run.

The lifecycle changes remain source/Android-tested and released, **not device
verified**. Resolve the preparation/navigation failure before another targeted
stop/follow-up attempt. Do not repeat successful 1695/1696 cases or claim a live
user-only-parent result from the offline tests.

## Shared Request Sequence

The subsequent [page command delivery recovery](chatgpt-command-delivery-recovery.md)
addresses proven-unsent commands after a missing bridge; it does not retry an
entered or ambiguous private write. Its release/device status is tracked there.

September 13: native/social sends and attachment reservations now allocate from
the existing `ChatGptWebObservedState` session sequence used by MCP commands.
There is still one send owner and one receipt cache, not another transport.

Previously the separate native `mcp_s<generation>` sequence overlapped the MCP
`mcp_<base36 sequence>` namespace: native `mcp_s1` equals MCP command 1009.
The retained-receipt watermark also interpreted native IDs as large MCP sequence
numbers. The actual receipt module reproduced `request_retired` for a new
`mcp_5` after 33 settled native IDs. This is a deterministic ownership bug;
it is not evidence that it caused the earlier fixture/navigation timeout.

The shared allocator survives document changes and send-owner replacement.
Busy/not-ready native clicks allocate nothing; native sends do not fabricate
pending MCP commands. Other providers keep their existing default allocator.
Focused tests cover 2,400 mixed read/send IDs, owner recreation and attachments.
The 53 JavaScript receipt/transaction/wiring tests passed with no skips in
`web-chat-shared-sequence-receipts-js-20260913-085523-990`.
Release Kotlin/Java compilation and 60 focused Android tests (six suites, no
failures or skips) passed in
`web-chat-shared-send-sequence-android-fixed-20260913-085336-935`.
The first compile attempt rejected access to the ledger's private default ID
factory; the corrected optional injection preserves the ledger default without
exposing or duplicating it. Only the corrected run is counted as passed.
This follow-on shipped in APK 1.1.1698 with page-command recovery. Cold and
background-return read-only commands passed; stop/follow-up still stopped before
sending at the same-route readiness boundary. No live send acceptance is claimed
for the shared sequence. See [the 1698 record](reports/chatgpt-command-recovery-1698.md).

## Not Completed

- Live follow-up after a stopped user-only turn and extended stopped-turn variants.
  The ordinary completed-turn follow-up and native stop above are accepted;
  cancelling a reader alone still **does not** confirm server stop.
- Live delayed-history/network-loss recovery, early handoff, anonymous SSE
  resume and additional handoff variants. The in-band authenticated WebSocket
  handoff is accepted; unknown variants retain their write barrier.
- Initial ownership without a mounted composer. This version removes the
  submission callback/readiness dependency but does not claim zero DOM bootstrap.
- New chats, tools, attachments, temporary/project/shared chats, non-personal
  workspaces and unobserved website asset profiles remain on existing paths.

Default enablement covers only the accepted scope above. It must not be expanded
from offline tests alone. Controlled trials remain tied to the owned transaction
and the exact reviewed provider profile.

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

## Read-Only Recovery Source Batch

September 13: `android_chatgpt_fresh_text_history_recovery_v1`, source-integrated,
offline verification only. `chatgpt_web_fresh_text_recovery.js` reuses the reviewed
`textHydrateHistory` fetch/apply contract instead of another sender or a second
transcript store. Adapter 369 assembles it; transaction v3 replaces an older
instance only after its pending writer has settled.

- A completed or interrupted local reader triggers one bounded recovery cycle.
  It checks the original conversation, message and parent, and retries an
  inconclusive history response at most twice, with 400 ms and 1,200 ms gaps.
  The whole cycle has a 15-second deadline; an HTTP failure ends that cycle.
- Foreground visibility, `pageshow` and `online` can resume a pending owner.
  Hidden documents do no automatic reads. Automatic work has a 10-second
  cooldown and three-cycle budget per turn; no interval or snapshot polling
  is introduced. Concurrent triggers share one job and one native finalization.
- The existing native conversation-refresh command joins this recovery while
  the independent writer is pending. It does not start generic prefetch in
  parallel or require the composer to be rendered/ready. Manual refresh can
  perform another bounded read after the automatic budget is exhausted.
- Reconciliation checks the registered account/document/branch both before
  fetching and before applying. A new runtime sender, regeneration, relay,
  Canvas generation or voice session prevents application. Explicit active or
  unknown server async status cannot masquerade as a completed reply.
- Confirmation finishes the native stream without resetting its text, schedules
  a snapshot and releases the next-send barrier. A new draft is left untouched.
  An uncertain original POST is never replayed; its old command receipt remains
  immutable, while authoritative history may later prove the turn completed.
- Stop preempts recovery. An uncertain stop may settle from terminal partial
  history; a user-only stopped branch still needs the stop acknowledgement.
  Document replacement cancels outstanding reads and invalidates late results.

This adds no new endpoint or credential channel and does not enable the fresh
sender as a production default. New chat/tool/attachment scopes, initial
composer-free ownership, active-stream resumption and actual device recovery
remain separate work. Navigation away suppresses recovery for that owner;
returning to its exact conversation permits recovery again. It is not yet a
multi-conversation independent-writer implementation.

The source/regression run passed **298 tests, zero failures and zero skips**:
`fresh-recovery-final-20260913-012824-171`. Coverage includes real module
composition for late history / runtime-writer races, recovery single-flight,
timeouts and late replies, foreground/online triggers, immutable write receipts,
native refresh routing, preserved drafts and the existing stream/stop/attachment
regressions. The public AST contract check ran against the retained reviewed
assets, without executing them or contacting a private account.

Android build, APK publication and real-account delayed-history/recovery
acceptance remain grouped with the other pending source batches; no phone
message was sent in this batch.
