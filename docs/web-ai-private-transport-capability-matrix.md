# Web AI private transport capability matrix

This document is the human-readable boundary for the runtime inventory exposed by
`elon.chatgpt_web.capability_matrix.v4`. The runtime catalog is authoritative for the
installed build; individual capability documents retain implementation evidence.

## Production defaults

| Capability | Provider | Result | Fallback |
|---|---|---|---|
| Conversation and project directory | ChatGPT | Completed and enabled | Official DOM directory |
| Conversation body prefetch | ChatGPT | Completed and enabled | Official WebView navigation |
| Send dispatch acknowledgement | ChatGPT | Completed and enabled | Official DOM confirmation |
| Streaming reply observer | ChatGPT | Completed and enabled; completion and sparse-watchdog timer/bridge device verified | Official DOM snapshot |
| Private stream completion settlement | ChatGPT | Completed, enabled, and device verified on `v1.1.1302 (1312)` | Official DOM snapshot |
| Realtime voice transcript refresh | ChatGPT | Completed and enabled; supervised voice exit pending | Retained native transcript and official DOM snapshot |
| Realtime voice managed native relay | ChatGPT | Completed and enabled; single native audio plus data-channel device verified | Official page-created WebRTC |
| Realtime voice live transcript stream | ChatGPT | Completed and production-default with native RTC; device event-shape acceptance pending | Same-origin conversation refresh and official DOM snapshot |
| Realtime voice background continuity | ChatGPT | Completed, enabled, and consumer default; native UI + persistent background WebView identity + native WebRTC media | Official page-created WebRTC, full-screen WebView voice, and foreground notification |
| Server API realtime voice experiment | 一龙 AI / OpenAI API | Implemented but disabled and hidden from consumer UI | Native UI + persistent background WebView identity + official WebRTC |
| Conversation directory | Google Web AI | Completed and enabled | Local cache and official page |
| Visited conversation body cache | Google Web AI | Completed, enabled, and device verified | Official WebView navigation |
| Stable conversation URL lifecycle | Google Web AI | Completed, enabled, and device verified on `v1.1.1317 (1327)` | Blank AI Mode plus native draft recovery |
| Reply stream and completion observer | Google Web AI | Completed, enabled, and stream-to-completion device verified on `v1.1.1303 (1313)` | Official DOM snapshot |
| Background navigation continuity | ChatGPT and Google Web AI | Completed, enabled, and device verified | Bounded official WebView recovery |
| Unified native send ledger | ChatGPT and Google Web AI | Completed and enabled; stable request-ID reconciliation targeted tests passed, device regression pending | Official-page reconciliation without automatic write replay |

All web-account transports keep the official page authoritative. They do not export
cookies, credentials, request headers, or private conversation content outside the
device. Every observer is bounded and emits nothing on malformed or unsuccessful
responses.

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

The coordinator never retries a write by issuing a second request. A queued command without
a receipt moves to `unknown/reconciling`, performs bounded read-only reconciliation, and then
asks the user to inspect the official page without restoring a potentially already-sent
draft. A command accepted by either transport forbids fallback write replay. A future
same-origin private sender may plug into this ledger only after it supplies a versioned
request contract and an explicit page-state handoff; current observers intentionally do not
capture enough request material, so ChatGPT and Google direct POST remain disabled.

Device acceptance on `v1.1.1313 (1323)` verified the production friend-chat surface with
ChatGPT adapter `188` and Google adapter `37`. Both providers completed an isolated exact
reply probe through the native composer; Google additionally exposed a streaming-to-complete
transition. The acceptance did not clear cookies or application data and emitted no private
conversation content.

Normal and interrupted realtime-voice exits now reuse the verified conversation-body
transport for a non-navigating refresh of the current `/c/{id}` only. Requests are
single-flight and inherit the existing timeout, cooldown, and circuit breaker. The
native transcript stays visible while the private snapshot and official DOM snapshot
race; failure emits no unsupported-capability error and the DOM path continues.

The native realtime peer also consumes the official WebRTC data channel directly for
allowlisted user and assistant transcript delta/final events. Bounded in-memory events
update native chat bubbles without reading the voice-page DOM. Event identifiers are
deduplicated, message and text sizes are capped, and MCP diagnostics expose only message
counts, never transcript text or payloads. A cold session starts with the documented
`oai-events` label, a current page-observed safe label overrides that preset, and a
server-created channel can replace a still-connecting local channel. A missing or changed
event shape is silent:
the existing same-origin conversation refresh and official DOM snapshot remain the
authoritative reconciliation path. Device acceptance must confirm the current private
event envelope before its verification marker advances beyond targeted tests.

Realtime voice identity and bootstrap remain owned by the persistent official WebView.
The default media session is owned by Android WebRTC, while the page-created WebRTC route
remains the failure fallback. A microphone foreground service keeps the session eligible
while the user opens another app. The
native chat orb is shown only while the owning ChatGPT conversation surface is active;
otherwise a system overlay and foreground notification provide continuity. Audio focus
automatically pauses for other media and resumes afterward. No WebRTC credential is
persisted or replayed. Device acceptance verified first-start handoff, Settings and
provider round trips, continuous recording, and media-focus pause/resume; direct human
touches on the system overlay pause and hang-up actions remain supervised acceptance.
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
