# Official runtime generation stop

Capability: `android_chatgpt_official_runtime_stop_generation_v1`.
Status: source implemented and offline verified, not device accepted or
completed. Adapter 304 adds disabled-composer lookup, stop-stage diagnostics and
partial-reply retention; see [the current correction](reports/chatgpt-stop-interruption-20260908.md). This is a
same-origin official-runtime bridge, not an independent Android HTTP transport.
Do not reimplement the source integration while device acceptance is pending.

## Scope and evidence

The existing native `chatgpt_stop_generation` command previously stopped only a
captured private request directly; ordinary official-runtime submissions still
required finding and clicking a DOM stop button. The new path reuses runtime
submit 5's conversation context and calls the pinned official stop function.
Readiness and pending attachments do not prevent capturing the conversation:
an in-progress response normally has an unready composer.

Public source, inspected without account credentials:

- [Composer](https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js),
  SHA-256 `9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`.
  `kOn` calls the current abort callback with `user_stop_mouse`; its callback
  reaches the conversation stop implementation. UI draft-restoration behavior
  is not copied into the native stop command, so a later draft is preserved.
- [Conversation](https://chatgpt.com/cdn/assets/conversation-small-hiw4wce20lu6te81.js),
  SHA-256 `296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`.
  Export `FVt` is async `mN`. It owns current stop-conduit/turn state, the
  feature-selected `/stop_conversation` request, local request cancellation and
  interruption-tree cleanup. The bridge passes the captured client thread and
  exact request ID with `clientInitiated: true`, not a copied POST template.
- [Shared state](https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js),
  SHA-256 `89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0`.
  `XM` and `HM.getRequestId` identify the request; `Fl` reads active-request
  membership; `Fx` reads conversation async status. The inspected `v7` enum is
  numeric: streaming 3, unread 4 and three realtime states 5 through 7.

All credentials remain inside the existing page identity layer. There is no new
endpoint guessing, credential export, proof replay, native system substitute,
microphone action or independent proxy change.

## Transaction rules

One committed conversation/controller/file-store context binds account,
document, route and request. The runtime modules must already be observed under
their pinned URLs; imports reuse browser module caching with a 1.5-second load
deadline. Context and request are checked again after loading and subscribing.
All three realtime states are rejected by the text stop command. Existing
captured-private-relay cancellation keeps its own owner and is tried first.

Duplicate pending stop commands share a transaction. While stopping, native
send and regeneration paths cannot start another writer. A stop response alone
is not success: the official operation must settle, the bound request must no
longer be active, and the conversation must be idle or completed-unread. A
different current request is not marked stopped. This does not control writes
made independently in another official-page client or device.

Observation uses existing subscriptions plus at most 30 in-memory checks at
100 ms after settlement, not recurring DOM scans. The total observation
deadline is 15 seconds. An unresolved official request retains ownership after
timeout; late settlement releases it after observation, even after the deadline.
Uncertain stop results never cause a second automatic stop or native completion
reset. Invocation exceptions or invalid runtime receipts retain the owner until
document replacement; protocol-mismatch recovery remains a device-check item.

Only a pre-invocation runtime-unavailable result may select the existing DOM
fallback. A one-use fallback claim checks the same document/account/conversation
and request again when the caller actually consumes the asynchronous receipt.
Navigation, changed identity or an already claimed duplicate cannot click a
different stop button. The complete official-page route remains available.

## Verification and remaining work

The eight initial production-wiring cases fail against the preceding source.
The final focused run passes 158 Node cases, with no skipped or cancelled tests,
across runtime stopping, production wiring, text submit, attachment submit and
regeneration. Tests cover duplicate commands, unready composer, request changes,
identity/route changes, all realtime states, load timeout, post-write timeout,
late settlement, bounded checks and one-use fallback claims. The actual Android
asset bundle also parses with the new module before its orchestrator.

The initial source batch had no APK. APK 1567 was subsequently installed but
used DOM fallback and lost the partial reply in its immediate snapshot. The
current correction's delivery and device status are in the linked report.
In the grouped production
UI acceptance, stop one long text response, preserve its partial answer, send
again, regenerate then stop, and confirm an active voice conversation is not
closed by a text-stop command. Verify official-request provenance and observed
completion, including network delay and conversation switching. Current pinned
module access, native feedback timing, latency and resource impact still require
device evidence; this document does not claim those checks passed.
