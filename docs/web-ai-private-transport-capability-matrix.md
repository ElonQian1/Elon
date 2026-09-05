# Web AI private transport capability matrix

This document is the human-readable boundary for the runtime inventory exposed by
`elon.chatgpt_web.capability_matrix.v4`. The runtime catalog is authoritative for the
installed build; individual capability documents retain implementation evidence.

## Production defaults

| Capability | Provider | Result | Fallback |
|---|---|---|---|
| Conversation and project directory | ChatGPT | Completed and enabled | Official DOM directory |
| Conversation body prefetch | ChatGPT | Completed and enabled | Official WebView navigation |
| Page-local identity context prewarm | ChatGPT | Completed and enabled; same-origin auth plus conversation refresh device verified | Observed official request context and persistent identity WebView |
| Conversation navigation receipt reconciliation | ChatGPT | Completed, enabled, and device verified on `v1.1.1399 (1420)`, adapter `218` | Official WebView navigation without write replay |
| Conversation pin transaction | ChatGPT | Completed, enabled, and device verified on `v1.1.1504 (1504)`, adapter `243`; formally shipped in `v1.1.1505 (1505)` | Official conversation options after explicit user choice |
| Conversation metadata mutations | ChatGPT | Completed and enabled; device verified on `v1.1.1506 (1506)`, adapter `244`, and formally shipped in `v1.1.1510 (1510)` | Official conversation options after explicit user choice |
| Send dispatch acknowledgement | ChatGPT | Completed and enabled | Official DOM confirmation |
| Streaming reply observer | ChatGPT | Completed and enabled; completion and sparse-watchdog timer/bridge device verified | Official DOM snapshot |
| Private stream completion settlement | ChatGPT | Completed, enabled, and device verified on `v1.1.1302 (1312)` | Official DOM snapshot |
| Realtime voice transcript refresh | ChatGPT | Completed and enabled; supervised voice exit pending | Retained native transcript and official DOM snapshot |
| Realtime voice managed native relay | ChatGPT | Completed and enabled; single native audio plus data-channel device verified | Official page-created WebRTC |
| Realtime voice live transcript stream | ChatGPT | Completed and production-default with native RTC; device event-shape acceptance pending | Same-origin conversation refresh and official DOM snapshot |
| Realtime voice background continuity | ChatGPT | Completed, enabled, and consumer default; native UI + persistent background WebView identity + native WebRTC media; foreground/background overlay controls device verified on `v1.1.1521 (1521)` | Official page-created WebRTC, full-screen WebView voice, and foreground notification |
| Server API realtime voice experiment | 一龙 AI / OpenAI API | Implemented but disabled and hidden from consumer UI | Native UI + persistent background WebView identity + official WebRTC |
| Conversation directory | Google Web AI | Completed and enabled | Local cache and official page |
| Visited conversation body cache | Google Web AI | Completed, enabled, and device verified | Official WebView navigation |
| Stable conversation URL lifecycle | Google Web AI | Completed, enabled, and device verified on `v1.1.1317 (1327)` | Blank AI Mode plus native draft recovery |
| Reply stream and completion observer | Google Web AI | Completed, enabled, and stream-to-completion device verified on `v1.1.1303 (1313)` | Official DOM snapshot |
| Background navigation continuity | ChatGPT and Google Web AI | Completed, enabled, and device verified | Bounded official WebView recovery |
| Snapshot-authoritative bridge handshake | ChatGPT | Completed, enabled, and device verified on `v1.1.1536 (1536)`, adapter `259` | Bounded official WebView session recovery |
| Unified native send ledger | ChatGPT and Google Web AI | Completed, enabled, and device verified on `v1.1.1520 (1520)` with ChatGPT adapter `247` and Google adapter `40` | Official-page reconciliation without automatic write replay |
| Same-origin text transaction coordinator | ChatGPT | Completed and enabled; native send/reply verified on `v1.1.1539 (1539)`, adapter `260`; current proof-bearing writes still use the official transaction. Direct private POST is not device verified | Official-page transaction without automatic write replay; [current contract and lifecycle evidence](chatgpt-same-origin-text-transaction.md) |
| Interaction presets and deferred chat actions | ChatGPT | Completed, enabled, and device verified on research build `v1.1.1367 (1388)` | Current official control and WebView navigation |
| Attachment upload reconciliation | ChatGPT | Completed, enabled, and device verified on published Release `v1.1.1374 (1395)`, adapter `207` | Official DOM attachment snapshot and bounded timeout |
| Native attachment upload progress | ChatGPT | Completed, enabled, and device verified on `v1.1.1491 (1491)`, adapter `239` | Indeterminate native status and official DOM attachment snapshot |
| Native image assets and cache-first gallery | ChatGPT | Completed, enabled, and device verified on Release candidate `v1.1.1375 (1396)`, adapter `208` | Bounded local image cache and official `/images` fallback |
| Native image-generation operation status | ChatGPT | Completed, enabled, and device verified on `v1.1.1518 (1518)`, adapter `247` | Official composer and `/images` page |
| Native private-response rich content | ChatGPT | Completed, enabled, and structurally device verified on Release `v1.1.1379 (1400)` for finance and line-chart cards | Official WebView rich content |
| Private composer dictation | ChatGPT | Completed and enabled; page-local identity, synthetic-audio endpoint proof, strict capture ownership, buffered transcription, draft reconciliation, timeout/circuit protection, and targeted integration tests passed | No automatic fallback; idle long press explicitly selects work-mode dictation |
| Selectable work-mode composer dictation | ChatGPT | Completed and enabled by direct reuse of the unchanged work-mode `AgentVoiceBridge`; explicit private/work selection user-verified on `v1.1.1483 (1483)` | No cross-mode fallback; idle long press switches back to private dictation |
| Private response read aloud | ChatGPT | Completed and production-default; same-origin streaming synthesis start/stop device verified on `v1.1.1498 (1498)`, adapter `241` | Official DOM read-aloud bridge or manual official page |
| Official response read aloud bridge | ChatGPT | Implemented as a non-default fallback; live official control discovery plus semantic/menu tests passed | Manual official page or explicit system-read-aloud selection |
| Android system response read aloud | ChatGPT | Implemented and enabled as an explicitly named alternate; audio start user-confirmed, stop acceptance pending | No automatic fallback |
| Native conversation management | ChatGPT | Completed and enabled; pin, rename, and archive mutations device verified through adapter `244` | Context-bound official options for share, files, delete, and repair |
| Native conversation project picker | ChatGPT | Completed, enabled, and device verified on `v1.1.1493 (1493)`, adapter `239`; the picker remains production UI and its previous DOM activation transaction is the compatibility fallback | Official conversation project menu |
| Private conversation project move | ChatGPT | Completed, production-default, and device verified on `v1.1.1514 (1514)`, adapter `245`; exact one-forward/one-restore round trip restored the original project | Context-bound official project-move coordinator |
| Acceptance evidence contract revisions | ChatGPT and Google Web AI | Completed, enabled, and installed-state migration verified on Release `v1.1.1393 (1414)`, adapter `212` | Retain implementation hashes as diagnostics without discarding accepted contracts |
| Compact `Pro` model control classification | ChatGPT | Completed, enabled, and production-surface device verified on Release `v1.1.1394 (1415)`, adapter `213` | Official model menu remains authoritative |
| Official feature sidebar trigger | ChatGPT | Completed, enabled, and production-surface device verified on Release `v1.1.1395 (1416)`, adapter `214` | Built-in native feature routes and the full official page |
| Role-aware official link classification | ChatGPT | Completed, enabled, and production-surface device verified on Release candidate `v1.1.1523 (1523)`, adapter `247` | Confirmation-gated official link invocation |

All web-account transports keep the official page authoritative. They do not export
cookies, credentials, request headers, or private conversation content outside the
device. Every observer is bounded and emits nothing on malformed or unsuccessful
responses.

Ordinary anchors that the official page reports with the generic `action` semantic are
now normalized to `open_link` only when their role is `link`. They remain confirmation
gated and have no placement in the consumer "current page operations" menu. An unknown
button keeps the generic `action` semantic and therefore still requests adapter review.
Three settled production-conversation samples on Release candidate `v1.1.1523 (1523)`,
adapter `247`, each reported 32 controls, eight classified links, zero generic controls,
zero unknown semantics, and no adaptation review. The completed capability is
`android_chatgpt_web_link_semantic_classification_v1`; it must not be reimplemented
without current regression evidence.

ChatGPT private reads, private dictation, and private read-aloud now share one page-local
identity context instead of waiting for an incidental official conversation request or each
fetching the session independently. The context prewarms `/api/auth/session` asynchronously at
document start, keeps authorization only in the WebView JavaScript closure, refreshes before
expiry, accepts newer authorization observed from official requests, and applies single-flight,
five-second timeout, and bounded circuit-breaker rules. It never persists or bridges the token.
On the authenticated Xiaomi WebView, a hot-injected production-path probe completed identity
prewarm plus the current project-conversation refresh in about 2.8 seconds, produced a native
message snapshot, and preserved the current route without a DOM read or write. The previous
one-second cold refresh ceiling was below the observed 2.2-2.7 second endpoint latency; the
bounded refresh budget is now 3-5 seconds while the existing native cache remains immediately
visible. The completed, production-default capability is
`android_chatgpt_page_local_auth_context_v1`; observed official request context and the
persistent identity WebView remain the fallback.

Device acceptance evidence is versioned by its user-visible case contract rather than by
every implementation or test-harness hash. The APK packages revisions for all 46 unique
cases and the build fails when the acceptance catalog and fingerprint map diverge. Existing
version 1 records migrate as contract revision 1; only an explicit revision bump invalidates
the affected case. Product input SHA-256 values remain in the structural snapshot as
diagnostic provenance and expose implementation drift without erasing a still-valid device
result. The completed, default-enabled infrastructure capability is
`android_chatgpt_verification_evidence_contract_revision_v1`. A read-only structural query
after the data-preserving Release upgrade reported 25 registered and 25 contract-current
historical records across 46 packaged cases. All 25 separately reported implementation hash
drift, proving that provenance remains visible without erasing accepted device evidence. The
query reported no private conversation content.

Conversation navigation now retains the normalized target identity across an official
document replacement. If the old page disappears before it can emit a command result, a
fresh authenticated snapshot completes that exact request only when its conversation
identity matches. An unrelated, expired, explicitly failed, or superseded request remains
failed or pending; no write or navigation command is replayed automatically. The stable
capability is `android_chatgpt_conversation_navigation_receipt_reconciliation_v1`.
Its targeted lifecycle tests pass and consolidated device acceptance is pending.

Conversation pinning now has a dedicated native transaction instead of navigating to the
target conversation and searching its visible menu. The page-local transport issues one
same-origin `PATCH` carrying only the desired starred state, waits for an HTTP success, and
then performs a read-only pin-directory reconciliation. Android changes the cached pin state
only after that acknowledgement; a transport timeout enters bounded read-only pin reconciliation
and never triggers an automatic write replay. A short authoritative pin override prevents stale
conversation-directory responses from reversing a confirmed result. The production menu
keeps the official conversation options as an explicit repair path. The stable capability ID
is `android_chatgpt_private_conversation_pin_v1`. Targeted transaction and UI policy tests pass;
the single-write pin/unpin round trip and stale-directory protection were verified on the
production friend-chat surface with APK `v1.1.1504`, adapter `243`.

Reopening the already-active ChatGPT or Google production friend-chat surface now keeps
the warm identity transport alive. Native composer, toolbar, and capability bindings are
refreshed, but the selected controller is not deactivated and reactivated. Switching to a
different provider still performs the normal bounded handoff. Smoke recovery likewise
waits for the existing session recovery coordinator instead of forcing an official page
reload, and the MCP harness reuses a healthy authenticated service before bootstrapping.
Device acceptance on APK `v1.1.1392 (1413)` repeated entry into the already-active
production ChatGPT surface, then verified that authentication, composer readiness, and
both page and adapter generations remained unchanged for ten seconds. It performed zero
remote writes and sent no messages. The completed, default-enabled capability is
`android_web_chat_active_provider_reentry_continuity_v1` and must not be reimplemented
without current regression evidence.

The ChatGPT composer can expose the current model as the compact label `Pro`, without a
model-family prefix. Adapter `213` routes that exact compact label through the shared model
policy used by both the production manifest and composer-option discovery. A read-only
production-surface device acceptance opened the already-discovered composer model control,
received the request-correlated control receipt, and observed one `Pro` control classified as
`model` with zero `Pro` controls classified as the generic action. The acceptance sent no
messages and dismissed the menu afterwards. The completed, default-enabled capability is
`android_chatgpt_compact_pro_model_control_v1`; it must not be reimplemented without current
regression evidence.

Feature discovery no longer treats a built-in native route as proof that the official sidebar
is already visible. Adapter `214` publishes the bounded built-in fallback immediately, but it
continues to the visible official sidebar trigger unless at least one live official feature node
is present. Production-surface device acceptance opened the sidebar, observed five official
feature nodes and one close control, dismissed it through the request-correlated command, then
confirmed the close control disappeared while the composer remained ready. It sent no messages
and changed no conversations. The completed, default-enabled capability is
`android_chatgpt_official_feature_sidebar_trigger_v1`; it must not be reimplemented without
current regression evidence.

The consumer default is one architecture: native chat and floating voice UI, the same
persistent background ChatGPT WebView as the identity and conversation/bootstrap layer,
and an Android-owned WebRTC peer as the media and live-transcript transport. The hidden
same-origin page performs a single-use in-memory offer relay without exporting cookies,
headers, credentials, SDP, ICE, or conversation content. The official page-created WebRTC
session remains the automatic fallback. The friend-chat phone action, blue composer voice
action, and semantic/MCP voice command all enter this route.
The separate server API realtime transport remains an explicit disabled experiment with
no consumer entry. The runtime inventory exposes both transport IDs, layers, visibility,
default state, and conversation scopes so future agents cannot conflate them. Details are in
`docs/native-realtime-voice-transport.md`.

Native attachments keep the official page as the upload owner. Adapter `207` arms a
document-start, same-origin observer only after the native picker is requested. It emits
only a version, monotonic sequence, state, and completed count. A successful completion
may release the already-reserved single send slot when the current official snapshot is
composer-ready and not streaming; DOM attachment chips remain valid evidence and the
existing timeout remains the fallback. This closes the official-upload/hidden-chip false
timeout without copying cookies into Android HTTP, replaying uploads, or exporting file
metadata. Device acceptance on `v1.1.1373 (1394)` completed one fixed-fixture upload and
reply, restored the original conversation, and registered
`supervised/attachment_lifecycle`. The completed capability is
`android_chatgpt_attachment_transport_reconciliation_v1` and must not be reimplemented
without current regression evidence.

The production composer now projects that existing redacted observer into a determinate
native upload status. Completed counts are monotonic, clamped to the locally selected total,
and refreshed without waiting for a full conversation snapshot. The sending phase is shown
separately from upload progress, so a stable official-page gate cannot look like a stalled
upload. Missing or malformed evidence keeps the previous indeterminate native status and the
official DOM attachment snapshot remains authoritative. This UI-only slice is registered as
`android_chatgpt_native_attachment_progress_v1`; filenames, bytes, credentials, and request
headers do not cross this progress contract. Release `v1.1.1491 (1491)`, adapter `239`,
completed a supervised fixed-fixture upload with visible native progress, one settled send,
and restoration of the original conversation. The capability is completed and remains
production-default.

Native image content keeps the official page as the identity, byte-fetch, generation,
and conversation authority. Adapter `208` maps allowlisted same-origin image content to
opaque handles; Android receives only a bounded JPEG preview and dimensions. The native
gallery renders the local cache immediately and starts a transient same-profile `/images`
sync only when the six-hour freshness marker is absent or expired. Manual refresh remains
an explicit forced sync. A sync imports at most 24 missing handles, while the cache remains
bounded to 80 files, 64 MiB total, and 1.1 MiB per file. It never emits source URLs, labels,
cookies, headers, credentials, or conversation text. Device acceptance imported 39 opaque
assets, opened the native viewer, returned to the same production chat, and proved that a
fresh second open did not create another sync WebView. The completed capability is
`android_chatgpt_native_image_asset_gallery_v1`; details are in
`docs/chatgpt-native-image-assets-gallery.md`.

The production composer projects image generation from existing authoritative state
instead of owning another generation transaction. Selecting the official image tool arms
the native operation banner; the official stream reports generation, and the opaque image
asset queue reports preview preparation or bounded preview failure. Once streaming has
ended and the preview queue is idle, the transient banner disappears while the selected
tool remains visible in its existing chip. Attachment progress keeps higher priority, and
unsupported or ambiguous states fall back to the official composer and `/images` page.
The completed capability is `android_chatgpt_native_image_generation_status_v1`. Xiaomi
production-surface acceptance on `v1.1.1518` / adapter `247` selected the official image
tool, sent exactly one isolated prompt, observed a completed assistant turn with a native
opaque `image` part, restored the original tool/conversation/draft state, and recorded the
verification case. Linked official image previews are classified as answer media instead
of citations, while small linked icons remain excluded. This flow must not be
reimplemented or retested without current regression evidence.

ChatGPT private response rich content now crosses the existing passive observer only as
the versioned `yilong.rich-content.v1` projection. Android accepts only bounded finance
and line-chart ASTs, finite numeric points, reviewed sources, and exact kind agreement;
malformed, oversized, credential-like, or unknown cards are dropped without rejecting
the surrounding message. Valid cards render inline in the production conversation and
open a native expanded detail surface, while the official WebView remains authoritative
for unsupported rich content. A fixed-data debug scenario exercised the same production
renderer on a Xiaomi device for finance, multi-series chart, and click-to-detail layouts
without reading a user conversation. The completed capability is
`android_chatgpt_private_rich_content_native_view_v1` and must not be reimplemented
without current regression evidence.

Production ChatGPT dictation is an explicit two-mode choice on the same white microphone.
The default after app process start is the verified same-origin private transport. A short
tap starts only the selected mode; an idle long press toggles between private dictation and
the existing work-mode `AgentVoiceBridge`. A start, submit, cancel, timeout, or asynchronous
failure never changes the selected mode and never starts the other recorder. There is no
automatic second-level fallback and the white microphone does not start an official DOM
third level. Existing official DOM session controls remain only so an official session that
was opened outside this entry can still be safely submitted or cancelled.

The first layer now uses the verified ordinary dictation contract: page-local identity,
`MediaRecorder`, and a bounded same-origin buffered transcription request. Credentials,
request headers, audio, and transcript never cross the WebView boundary; only lifecycle
receipts and the reconciled draft reach Android. A start is accepted only after capture is
confirmed, and a submit completes only after a fresh official composer snapshot contains
the new draft. Any failure stays owned by the selected layer and returns that layer to an
idle/retryable state; it cannot be interpreted as permission to start another transport.
The stable capability IDs are `android_chatgpt_private_dictation_transport_v1` and
`android_chatgpt_native_dictation_v1`; both are production enabled and device verified.
Release `v1.1.1483 (1483)` received user-supervised acceptance for the explicit private/work
selection, including the absence of automatic cross-mode fallback.

The rejected `1.1.1470` experiment must not be reintroduced as private composer dictation.
It reused the full realtime-voice `/realtime/wm` takeover: Android microphone capture,
peer connection, and the data channel all became ready, but the channel produced no
composer transcription event and the official socket later emitted a voice-conversation
commit. That behavior creates a voice turn instead of a draft and therefore cannot own the
production microphone button. The experimental engine and its production wiring were
removed. Research builds now observe only bounded session-profile and data-channel event
shapes so the actual official dictation mode can be distinguished without recording audio
or recognized text.

Assistant response read-aloud now exposes the same-origin website voice as the first native
action instead of silently substituting Android TTS. The stable production capability is
`android_chatgpt_private_response_read_aloud_v1`. The persistent identity WebView copies its
page-local runtime authorization only into a same-origin `GET /backend-api/synthesize` request;
credentials and headers never cross into Android. The response body is consumed as a bounded
stream and appended to `MediaSource`, so playback starts after the first usable audio blocks
instead of waiting for the entire answer. The transport owns one context at a time, supports
immediate stop, uses separate response-header, stream-stall, buffer, and playback-start
timeouts, and opens a short circuit after repeated failures. Release `v1.1.1498 (1498)`, adapter
`241`, started playback in about 4.5 seconds through the production MCP path and stopped in
under one second; the following typed snapshot reconciled to `idle`.

The previous `android_chatgpt_official_response_read_aloud_bridge_v1` remains a non-default
fallback. It opens the exact context-bound official message menu in the persistent identity
WebView, discovers the official `read_aloud` control with bounded polling, and invokes it. A
missing transient DOM node is shown only as preparation and is never reported as an absent
website capability. The manual official page remains the final repair path.

`System read aloud` remains a separately named, non-default alternate backed by the existing
`VoiceSpeaker`. It splits the complete answer into bounded sentence chunks, supports stop,
does not log or persist text, rejects stale completion callbacks, rebuilds after failures, and
uses a bounded watchdog. Selecting either mode stops the other when its current state is known;
there is no automatic cross-mode fallback and no hidden long-press mode switch. The stable
system capability remains `android_chatgpt_native_response_read_aloud_v1`. Audio start was
user-confirmed on the previous installed build; explicit stop acceptance remains pending.

Conversation rows expose a native project destination picker backed by the bounded
project-directory cache. The coordinator navigates to the exact conversation, opens its
context-bound official options, resolves one exact project title in the official project
chooser, activates that exact official control once, and handles at most one matching
second-stage confirmation. Recovery is armed before that one write. An absent or timed-out
receipt never replays it: scoped membership probes and low-frequency full-directory refreshes
reconcile the result, while only fresh, unique path-and-project agreement can report success
or prove that the conversation remains at its source. Ambiguous results remain pending with
the official menu available. Release `v1.1.1493 (1493)`, adapter `239`, completed one
reversible device round trip with exactly one forward and one restore write, no cleanup
write, no unknown recovery state, and the conversation restored to its original project.
The completed, production-default capability is
`android_chatgpt_native_conversation_project_move_v1`.
Its native picker is reused by the private transaction below; the official DOM activation is a
compatibility fallback, not a second default write path.

Conversation pin/unpin, rename, and archive/unarchive now share one versioned page-local
metadata transaction. Each explicit user action issues exactly one same-origin `PATCH`,
then settles through bounded read-only directory reconciliation; timeouts never replay the
write. A confirmed archive installs an explicit native directory tombstone immediately so
a stale cached row cannot reappear while the official directory refresh is in flight. The
production current-conversation entry resolves the native conversation identity first and
uses the same coordinator as a directory row. Sharing, files, delete, and repair remain in
the exact context-bound official options; project move retains its separately verified
official activation transaction. The completed capability is
`android_chatgpt_private_conversation_metadata_mutations_v1`. APK `v1.1.1506 (1506)`,
adapter `244`, completed unarchive, rename-forward, rename-restore, archive-restore, and
explicit-tombstone acceptance on a dedicated test conversation. The title and archived
state were restored, the app returned to conversation home, and no message, Cookie clear,
or app-data clear occurred. Formal APK `v1.1.1510 (1510)` was then installed without clearing
data; MCP confirmed adapter `244`, a ready authenticated bridge and composer, and restored
conversation home. See `docs/chatgpt-private-conversation-metadata-mutations.md`.

Conversation project move now reuses the same versioned page-local mutation boundary. The
native cached project picker submits one same-origin project-membership `PATCH`, then confirms
the result using read-only conversation metadata or the selected project directory. A timeout
never replays the write. The older context-bound DOM coordinator remains available only as the
explicit official fallback. The completed, production-default implementation capability is
`android_chatgpt_private_conversation_project_move_v1`, adapter `245`; targeted transaction,
bridge, capability, and production UI tests plus the Release build passed. Release
`v1.1.1514 (1514)` then completed a reversible MCP-only device round trip with exactly one
forward write and one restore write. Read-only reconciliation confirmed that the original
project membership was restored, with no unknown recovery state, private-content output,
Cookie clear, or app-data clear. The accepted receipt lifetime is 35 seconds and the production
coordinator waits up to 40 seconds, covering the roughly 30-second worst-case private
authentication/write/reconciliation budget without replaying the write. See
`docs/chatgpt-private-conversation-project-move.md`.

The Google conversation directory persists the timestamp of the last successful
official directory response. Cached rows render immediately; a legacy or expired cache
is then refreshed in the background, while a recently verified cache does not trigger
another DOM read every time the side menu opens. The version 1 cache remains readable
and migrates as stale instead of being discarded or misreported as freshly verified.

Google prompt execution URLs and durable conversation URLs have separate contracts.
An official `/search` URL without `csuir` may execute the current `q`, but it cannot enter
the conversation directory, visited-body cache, restart pointer, history-open coordinator,
or official fallback. Only a bounded allowlisted URL with a non-empty `csuir` is durable.
Legacy prompt URLs are filtered during cache restore without clearing cookies or app data;
recovery opens blank AI Mode instead of executing an old prompt again.

Previously visited Google conversation bodies use a provider-scoped, URL-validated
snapshot before official navigation starts. The cache is bounded to 30 days, 128 files,
24 MiB total, and 2 MiB per conversation. A missing, expired, corrupt, or mismatched
snapshot renders an empty loading state and continues through the official page.

Device acceptance on APK `v1.1.1297 (1307)` verified the cache-first path without reading
or exporting conversation text: reopening the active visited Google conversation returned
four cached message records while the official page was still `loading`; the official page
became composer-ready about 1.1 seconds later with the same four records. Provider switching
also restored the 14-row cached directory before the official composer was ready. This
capability is complete and must not be researched or reimplemented again without current
regression evidence.

Google reply observation keeps fast bounded snapshots only until the matching assistant
reply appears. Once streaming is visible, MutationObserver and the passive same-origin
completion signal remain primary while dense polling is replaced by four sparse watchdogs
over 68 seconds. Repeated streaming snapshots cannot stack timers, completion cancels the
remaining watchdog immediately, and the official DOM snapshot remains the fallback.

Adapter version 40 records the visible answer baseline immediately before each send. It rejects
a transient second-turn DOM answer when that answer matches the baseline and the bounded observer
has captured the reply for the current prompt. The observer result replaces only that proven
carry-over; any distinct current DOM answer remains authoritative. No direct Google POST is
introduced. If the clipped DOM omits the current prompt entirely, the adapter emits a
generation-scoped prompt/reply pair for native text-based merging only while the stable Google
conversation URL identity still matches the send-time identity.

Device acceptance on APK `v1.1.1303 (1313)` used one isolated exact-marker conversation
without exposing message content. The native Google Web AI surface entered streaming,
then returned to a completed ready state with one assistant reply, and restored the
original conversation. This closes the adaptive-watchdog regression gap: the verified
streaming snapshot exercises the dense-to-sparse transition, while completion exercises
the watchdog cancellation path. The smoke reports only structural booleans and counts and
does not clear cookies or application data.

ChatGPT streaming remains event-driven while the verified private response observer is
producing the current turn. Private progress schedules the native snapshot directly,
duplicate DOM mutations are ignored during that interval, and a four-second read-only
watchdog preserves official DOM reconciliation if private progress stalls. When no private
stream is available, the existing 400 ms bounded DOM heartbeat remains unchanged.

Device acceptance on APK `v1.1.1302 (1312)` verified completion settlement without
reading or exporting message text: a fresh native send entered streaming, the private
observer revision advanced to a completed response, native streaming became false, and
the original conversation was restored. A completed private response now releases stale
native streaming immediately when the official stop control is absent; a still-visible
official stop control remains authoritative. Each new send clears only the previous
private completion snapshot before the official click, without replaying or duplicating
the request. This capability is complete and must not be reimplemented without current
regression evidence.

APK `v1.1.1310 (1320)`, adapter `188`, closed the separate sparse-watchdog timer and
native-bridge regression gap with a no-request structural device fixture. It exercised the
production policy algorithm in an isolated policy instance, fired after 5.528 seconds, settled
native streaming, and restored the original conversation and provider. The fixture did not
dispatch or replay an official request, clear cookies or application data, or emit private
content. This evidence verifies the watchdog timer and bridge, not a naturally occurring sparse
external ChatGPT stream; runtime failures still fall back to the official DOM snapshot.

Realtime voice hangup has two separate states: the APK requesting the official action,
and the official call actually settling. Late reconciliation runs only after the official
hangup control accepted the command. A missing or temporarily unreadable control keeps
the compact surface in `still in call` state and cannot silently close the backing session.

Realtime voice activation also separates permission from current-call evidence. A fresh
WebView microphone grant still confirms first launch, while a reused grant is accepted only
when the current adapter exposes the official hangup control. This avoids waiting for a
permission counter that cannot change on later starts without treating permission alone as
proof that a call is active.

Background provider switching keeps one already-started official navigation alive.
On resume, the APK first reattaches the versioned page adapter and cached snapshot;
only a failed or stalled document consumes the bounded full-page reload budget. It
does not add a new request path or keep an idle polling loop alive.

Text sending enters both providers through one single-owner command ledger and an explicit
official-page transport port. Each command has a stable bounded request ID, authority,
acceptance state, page-sync state, generation and bounded completion history. Provider
controllers retain only provider-specific readiness messages and rendering callbacks.
Page command results settle only the matching request ID, so a delayed receipt cannot settle
a newer message. Observable page/reply evidence must still be newer than the pre-send
snapshot, so an older identical prompt cannot settle a repeated send.

Production device acceptance on `v1.1.1520 (1520)` exercised the native friend-chat surface
against ChatGPT adapter `247` and Google adapter `40`. Each provider created an isolated
conversation, dispatched exactly one marker prompt through `set_input_text` plus `send_input`,
observed one completed assistant turn, and restored the original conversation. Both reports
confirmed that cookies and application data were preserved and no private conversation
content was emitted.

The coordinator never retries a write by issuing a second request. A queued command without
a receipt moves to `unknown/reconciling`, performs bounded read-only reconciliation, and then
asks the user to inspect the official page without restoring a potentially already-sent
draft. A command accepted by either transport forbids fallback write replay. A future
same-origin direct ChatGPT sender is now connected through a versioned pure-text transaction.
It is eligible only for a current, route-bound, stream-confirmed request template without
one-time dynamic proof. Current ChatGPT Web sends contain such proof, so adapter `206`
rejects the direct path before dispatch and immediately invokes the official page transaction.
The 15-second transport timeout, two-failure 45-second cooldown, explicit stop semantics, and
read-only reconciliation remain active if a future page contract exposes a reusable template.
Google direct POST remains disabled.

ChatGPT model and tool choices use built-in presentation presets plus a user-scoped,
bounded stale-while-refresh cache. The native UI renders immediately and resolves the
current official semantic ID only when the user selects an option. Temporary chat uses
the same rule: an unobserved control remains actionable, one desired state is queued,
and success is shown only after the live official control confirms it. Starting a new
conversation is also accepted while the background session is idle or loading; the APK
shows the blank native conversation immediately and dispatches the official navigation
once the current bridge is ready. Duplicate taps cannot issue a second navigation.

Device acceptance on research APK `v1.1.1367 (1388)` started from an existing conversation,
force-stopped only the process, and requested a new conversation while the production
surface reported `loading`. The intent was accepted, the official path crossed to a blank
composer-ready conversation, and the original conversation was restored. A separate blank
conversation verified that the temporary-chat preset remained clickable before relying on
the live DOM, the official selected state changed, the original state was restored, and the
original conversation reopened. The evidence contains only structural booleans and does not
clear or export cookies, application data, credentials, headers, or conversation text.

Device acceptance on `v1.1.1313 (1323)` verified the production friend-chat surface with
ChatGPT adapter `188` and Google adapter `37`. Both providers completed an isolated exact
reply probe through the native composer; Google additionally exposed a streaming-to-complete
transition. The acceptance did not clear cookies or application data and emitted no private
conversation content.

Active, normal-exit, and interrupted realtime voice reuse the verified conversation-body
transport for a non-navigating refresh of the current `/c/{id}` only. Requests are
single-flight and inherit the existing timeout, cooldown, and circuit breaker. Native
data-channel events have priority; before one is parsed, private reconciliation runs about
every 1.5 seconds, then backs off to about every 6 seconds. The native transcript stays
visible while authoritative snapshots update it. A DOM snapshot is only a sparse watchdog
about every 12 seconds and an exit fallback; failure emits no unsupported-capability error.

The Win managed WebView2 peer now publishes a route-bound structural lifecycle version 2.
A blank-chat `/` route may adopt the first official `/c/{id}` assigned during voice startup,
while a later cross-conversation route still closes the peer. Microphone acquisition, offer,
relay, answer, connection, remote-audio, mute, fallback, and close state can drive the native
control surface without retaining SDP, ICE, credentials, headers, audio, or transcript text.
A relay that answers but never connects is released after a bounded timeout and the official
page-created voice path remains available.

The Win native chat surface also owns an independent realtime voice control dock. Once
the managed peer starts acquiring media, mute and hang-up no longer wait for the official
DOM control inventory to refresh: local media is controlled first, then the official state
is reconciled when its control is available. The dock shows only structural microphone,
remote-audio, and private-transcript evidence. A missing official hang-up confirmation still
uses the bounded watchdog and is never reported as a successful close prematurely.

The native realtime peer also consumes the official WebRTC data channel directly for
allowlisted user and assistant transcript delta/final events. Bounded UTF-8 JSON frames are
accepted whether WebRTC marks them as text or binary. Bounded in-memory events
update native chat bubbles without reading the voice-page DOM. Event identifiers are
deduplicated, message and text sizes are capped, and MCP diagnostics expose only message
counts, never transcript text or payloads. A cold session starts with the empty-label
shape observed from ChatGPT Web rather than the public Realtime API's `oai-events`
example. A current page-observed safe label overrides that preset, and bounded local plus
server-created channels stay observed until close. A missing or changed event shape is
silent: the existing same-origin conversation refresh and sparse official
DOM watchdog remain the authoritative reconciliation path. The managed production entry
initializes the existing transcript-continuity owner, so authoritative snapshots can update
native bubbles without an empty DOM clearing retained content. Device acceptance must
confirm either live data-channel events or visible private-snapshot reconciliation before
the relevant verification marker advances beyond targeted tests.

Realtime voice identity and bootstrap remain owned by the persistent official WebView.
The default media session is owned by Android WebRTC, while the page-created WebRTC route
remains the failure fallback. A microphone foreground service keeps the session eligible
while the user opens another app. The
native chat orb is shown only while the owning ChatGPT conversation surface is active;
otherwise a system overlay and foreground notification provide continuity. Audio focus
automatically pauses for other media and resumes afterward. No WebRTC credential is
persisted or replayed. Device acceptance verified first-start handoff, Settings and
provider round trips, continuous recording, and media-focus pause/resume. On
`v1.1.1521 (1521)`, real production-surface controls also verified the foreground orb
expand and hang-up path plus the background system orb expand, pause, resume, return-to-app,
and hang-up paths. Both hang-up paths reached a terminal native peer state and stopped the
microphone foreground service without clearing app data or login state.
An unconfirmed official hang-up remains an ongoing call, receives one bounded automatic
retry, and collapses to the non-blocking native orb instead of being misreported as a
connection failure or leaving a large action card over the conversation. Existing
versioned page snapshots now confirm a late official exit immediately after a two-second
stability window. A single-flight sparse watchdog performs only ten read-only checks over
at most two minutes when no snapshot event arrives; it never clicks hang-up again, and a
late official exit automatically closes the orb and reuses the existing conversation
refresh path.

## Audited non-capabilities

- Google direct send is not implemented. The official form/navigation path already
  confirms dispatch from navigation, composer, query, or streaming state; a second
  request could split official page state from server state.
- Google direct conversation-body prefetch is not implemented because controlled endpoint
  inventory has not identified a safe same-origin detail endpoint. The completed visited-body
  cache remains stale-while-revalidate and is always followed by official navigation.
- ChatGPT realtime voice does not persist or replay live WebRTC credentials. The APK
  caches the loaded WebView session and per-conversation launch hints, then creates a
  fresh native peer from a single-use same-origin bootstrap for each start. Unknown or
  failed bootstrap shapes use a fresh official page-created connection instead.
- Redacted research probes remain disabled in production. They are not user features
  and require a new compatibility question before they may be re-enabled.

## Delivery rule

Completed entries are not reimplemented without regression evidence. A deferred or
audited entry may change only after a new controlled observation proves a safer and
faster official same-origin path with automatic fallback.
