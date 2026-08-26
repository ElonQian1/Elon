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
| Realtime voice background continuity | ChatGPT | Completed and enabled; native/system overlay handoff and media auto-pause device verified, manual overlay actions pending | Official WebView voice and foreground notification |
| Native API realtime voice | 一龙 AI / OpenAI API | Implemented and enabled for an explicit new local conversation; targeted tests passed, device verification pending | Official ChatGPT WebView voice |
| Conversation directory | Google Web AI | Completed and enabled | Local cache and official page |
| Visited conversation body cache | Google Web AI | Completed, enabled, and device verified | Official WebView navigation |
| Reply stream and completion observer | Google Web AI | Completed, enabled, and stream-to-completion device verified on `v1.1.1303 (1313)` | Official DOM snapshot |
| Background navigation continuity | ChatGPT and Google Web AI | Completed, enabled, and device verified | Bounded official WebView recovery |

All web-account transports keep the official page authoritative. They do not export
cookies, credentials, request headers, or private conversation content outside the
device. Every observer is bounded and emits nothing on malformed or unsuccessful
responses.

The separate native API realtime transport is not a ChatGPT web-account shortcut. It
uses the existing authenticated 一龙 realtime WebSocket and records to local 一龙 AI
history. Its production entry is deliberately labeled `原生实时 AI`, while the blue
ChatGPT composer voice action remains owned by the current official web conversation.
The runtime inventory exposes both transport IDs and their conversation scopes so UI,
ADB, MCP, and future agents cannot conflate them. Detailed boundaries are in
`docs/native-realtime-voice-transport.md`.

The Google conversation directory persists the timestamp of the last successful
official directory response. Cached rows render immediately; a legacy or expired cache
is then refreshed in the background, while a recently verified cache does not trigger
another DOM read every time the side menu opens. The version 1 cache remains readable
and migrates as stale instead of being discarded or misreported as freshly verified.

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

Normal and interrupted realtime-voice exits now reuse the verified conversation-body
transport for a non-navigating refresh of the current `/c/{id}` only. Requests are
single-flight and inherit the existing timeout, cooldown, and circuit breaker. The
native transcript stays visible while the private snapshot and official DOM snapshot
race; failure emits no unsupported-capability error and the DOM path continues.

Realtime voice remains owned by the official WebView and WebRTC session. A microphone
foreground service keeps the session eligible while the user opens another app. The
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
  caches the loaded WebView session and per-conversation launch hints, then lets the
  official page create a fresh connection for each start.
- Redacted research probes remain disabled in production. They are not user features
  and require a new compatibility question before they may be re-enabled.

## Delivery rule

Completed entries are not reimplemented without regression evidence. A deferred or
audited entry may change only after a new controlled observation proves a safer and
faster official same-origin path with automatic fallback.
