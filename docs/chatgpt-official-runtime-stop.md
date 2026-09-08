# Official runtime generation stop

Capability: `android_chatgpt_official_runtime_stop_generation_v1`.
Status: stop runtime v7 is published in APK 1576 and its ordinary guest stop
operation is device verified (`official_runtime_v1:stop_observed`, partial text
retained). The full capability is not completed: follow-up still combines the
two synthetic prompts and replaces the partial answer. Do not repeat request-ID
research; that guard is now proven compatible. See the
[1576 evidence and stream-visibility correction](reports/chatgpt-stop-followup-1576.md)
for the remaining boundary. A runtime stop receipt does not prove turn continuity.
Earlier adapter 304 added
disabled-composer lookup and partial-reply retention; see
[the earlier correction](reports/chatgpt-stop-interruption-20260908.md). This is a
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

### Composite official request IDs, 2026-09-08

APK 1575 (source `72be1524bc62bcac486ec53cca375875e1b2439b`, SHA-256
`10dac1fb35784d30fdc73a809d4ce666f96b9f72c1553ae9cdf00aa4b214e18b`)
was published and installed with `-r`. The native owner diagnostic now works.
The complete guest run reports `cached=true`, `tree=true`, `mode=streaming`,
`request=invalid` and `generation=false`. Stopping preserves the partial answer
(139 characters before, 199 after), but uses DOM fallback. Follow-up produces
one combined user row and one replacement assistant row in both native and
web projections. Official and native drafts are empty throughout, excluding a
leftover editor draft as the source of concatenation. The original empty view
and awake setting were restored. Evidence:
`stopped-turn-device-1575-20260908-191321-479`.

The current public composer `8b34dbc2-nhot65scqrg20d6p.js` has SHA-256
`36644eb82aac9c399bce384c18140f8c878dd780c8f787440b80f27971729733`.
Both ordinary submission and resume build a request ID from the `request-`
prefix, the opaque client-thread key, and a counter. The composer imports that
counter as `Jte`; shared export `fN` resolves to `o$e`, which increments `b$e`.
The shared source hash is recorded in `chatgpt-private-runtime-bindings.md`.
The former message-ID character/length restriction is not this request contract.

Runtime v7 accepts that composite form up to 512 characters and rejects control
characters or a missing numeric suffix. Previously accepted simple IDs remain
supported. Server, message and turn-ID validation is unchanged. The exact live
request is still pinned through context, subscription and invocation; replacing
it cannot stop a successor. Only observed official cleanup permits the next send.
The request remains inside the page and is neither logged nor copied into an
independent HTTP request. APK 1576 has now verified this narrow compatibility
fix on-device; follow-up isolation remains failed, as recorded above.

Four composite dispatch cases and successor coverage failed against v6. The
focused stop/ownership/diagnostic suite passes 129 cases with no skips after the
change, including control-character, size and non-protocol rejection. Evidence:
`stop-composite-request-green-20260908-192051-494`.

### Grouped release, 2026-09-08

Release `1.1.1574` (code 1574), source
`8c1b974862e73896903c723eb1d4eaebe9063a54`, compiled and published successfully.
APK SHA-256:
`341cbf0cdb108632ce3a24846bab741977b870bb3ded00544fb5033fd4f3fa8c`.
The server version receipt agrees, and all 100 ordered adapter assets inside
the APK match their source files, including stop v6 and probe v16. Build log:
`web-chat-stop-gallery-grouped-release-20260908-182738-650`.

Publication initially skipped autodeploy while a separate app held the screen.
After the user explicitly asked to continue, `adb install -r` installed 1574 and
MCP opened the production social-chat surface. The app retained its guest session;
no data or identity was cleared. A synthetic long reply used
`official_runtime_v1:accepted`. Stopping preserved 252 characters (422 after
settlement), but its empty detail and `request_unavailable` diagnostic identify
DOM fallback, not the required runtime stop. The next owner probe failed with
`invalid_probe_mode`, so the harness did not send a follow-up. It restored the
empty original view and awake setting. Evidence:
`stopped-turn-device-1574-20260908-184536-042`.

### Native owner-diagnostic gate

Page probe v16 already implements `stop_runtime_owner`, but native
`ChatGptWebPrivateProtocolEvidence.MODES` omitted it and its receipt validator
did not accept `elon.stop_runtime_owner.v1`. Source now connects both boundaries.
Only the exact schema, three Boolean fields and fixed request/mode enums pass;
unknown fields, identifiers, text and type coercions remain rejected. This adds
no live capture, network request, stop invocation or credential access.

A new cross-layer gate test fails against the preceding native source. The
focused page/stop suite then passes 73 cases without skips. Release compilation
and the focused native protocol suites also pass; evidence stem:
`stop-owner-native-gate-android-tests-20260908-185142-334`. Native tests cover
the complete command-result parser as well as typed-shape rejection. This is a
diagnostic wiring correction, not evidence that the stop regression is fixed.
The next device run must obtain the owner shape and complete the follow-up case.

### Conversation-owned stop, source batch 2026-09-08

Runtime v6 supports the current official stop overload when a streaming server
conversation has no request ID. It requires the already cached shared module,
a server-backed conversation, a valid current turn and leaf, and the exact
streaming status object. It rejects a tree already being interrupted. The
turn, leaf and status identity are pinned before loading or subscribing and
checked immediately before calling the same versioned `FVt` export with its
default request argument. No additional endpoint or credential transport is
introduced. A new request, turn, leaf or status generation cannot inherit the
old stop; all three realtime modes are rejected without a DOM fallback.

The underlying source is the inspected 2026-09-07 conversation bundle listed
in `chatgpt-private-runtime-bindings.md`: export `hHt` resolves to async `BM`.
Its streaming-conversation branch can run without active-request membership.
The existing shared exports provide `HM.getCurrentLeafId`,
`HM.getConversationLastTurn`, and the stored `Fx` async-status object. The
implementation reuses those exports rather than adding new minified aliases.

Stopping remains pending after the official promise resolves while generation
is still streaming. Only idle/completed-unread cleanup of the same turn and
leaf releases normal send/regenerate controls. Timeout, identity changes and
uncertain results do not replay the stop. The original request-owned path and
one-use pre-invocation fallback claim are retained.

MCP `chatgpt_private_protocol_probe` with mode `stop_runtime_owner` now returns
only cached-runtime, request-shape, tree-presence, generation-bound and mode
fields from the last capture. Reading it performs no live capture, imports,
network requests or stop action. It contains no conversation/request IDs,
credentials or text, and resets its visible shape after document replacement.
Probe reinjection upgrades existing observers without stacking them.

The new request-less tests failed against v5 before implementation. The final
focused suite passes 139 cases, without skips, across request/conversation
ownership, guest behavior, runtime bindings, diagnostics and native command
wiring. The ordered 99-asset Android script bundle parses successfully.
This is source/offline evidence only: no new Android build or installation was
performed for this source batch, per the grouped-release workflow. The phone
was unlocked but in a separate grid-creation activity; only readiness and
foreground metadata were read. That activity was not interrupted.

Next grouped device check: from the production native chat, stop a synthetic
long reply, record both stop receipt and `stop_runtime_owner`, and send a short
follow-up. Require `official_runtime_v1:stop_observed`, preservation of the
partial reply, two separate user/assistant turns, and no draft loss. If it
fails, use the recorded owner shape to distinguish missing cache, missing tree,
missing turn/leaf, invalid request format and non-streaming official state.
The APK 1570 failure below remains authoritative until that check passes.

### Current-request correction, 2026-09-08

APK 1569 / adapter 305 device acceptance reproduced `request_unavailable`
and DOM fallback on a fresh guest long reply. The received partial answer
survived stopping (119 characters before, 201 after). A following native send
then projected one combined user row and one replacement assistant row, not
two separate turns. An ordinary two-turn control retained four separate rows.
The stop/follow-up case therefore remains failed, not completed.

Stop runtime v5 now reads `HM.getRequestId(XM(conversation.id))` from the already
loaded versioned shared runtime. The current public composer uses that same
tree selector to populate its render-time `currentRequestId` property. An
available live tree, including an empty request, takes precedence over that
property. The request is pinned synchronously before any load/subscription;
all subsequent ownership and replacement checks use the same live source.
Cold unavailable state does not queue a late stop or import a module to select
a later request. No DOM click, credential copying or new HTTP endpoint is added.

`stop-current-request-red` failed 10 of 11 new cases against v4;
`stop-current-request-tests` passes 73 focused cases (including the 11 new
cases), with no skips. This proves the stale-property source gap, not that it
is the device root cause.

APK 1570 / adapter 306, source `5fa473e27`, was built, published and installed
without clearing data. Its SHA-256 is
`bee2ee9d5b66190df34c2e8d1ec97e9d9a76c75ee093ae06d1c62339a7811acf`.
The v5 device rerun still reports `request_unavailable` and uses DOM fallback.
The partial answer survived stopping (229 characters before, 274 after), but
the next native send again produced two rows instead of four: a combined user
row and a replacement assistant row. All four stop/follow-up assertions failed;
the harness now exits nonzero on assertion failure. This correction did not
resolve the live failure and must not be marked completed.

Current public-source evidence offers a narrower next investigation: the
versioned conversation export `hHt` resolves to `BM`, which defaults its request
argument from the tree but can also stop a streaming conversation without an
active request entry. It conditionally uses the official conversation-stop
path. Our runtime rejects a missing request before reaching that function.
This is a compatibility hypothesis, not proof of the phone's active route.
Do not simply remove the request guard: any alternative owner must still pin
the same generation before asynchronous work, exclude voice, and observe
completion. A subsequent MCP protocol-shape diagnostic was stopped by the
foreground guard before capture or sending; no endpoint/status evidence was
obtained from that attempt. The app was not brought back over the user's screen.

New logs: `stopped-turn-device-1570` (failed acceptance) and
`stopped-turn-protocol-1570` (foreground guard, not a protocol result).
The device logs are `two-turn-device-1569-followup` and
`stopped-turn-device-1569`; they contain only synthetic structural results.
The temporary awake setting was restored. The earlier remote test screenshot
was removed after reconnection. No user messages or credentials were exported.

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
