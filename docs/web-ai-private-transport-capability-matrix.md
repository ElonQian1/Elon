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
| Streaming reply observer | ChatGPT | Completed and enabled; adaptive-watchdog device regression pending | Official DOM snapshot |
| Realtime voice transcript refresh | ChatGPT | Completed and enabled; supervised voice exit pending | Retained native transcript and official DOM snapshot |
| Realtime voice background continuity | ChatGPT | Completed and enabled; native/system overlay handoff and media auto-pause device verified, manual overlay actions pending | Official WebView voice and foreground notification |
| Conversation directory | Google Web AI | Completed and enabled | Local cache and official page |
| Visited conversation body cache | Google Web AI | Completed and enabled; device acceptance pending | Official WebView navigation |
| Reply stream and completion observer | Google Web AI | Completed and enabled; adaptive-watchdog device regression pending | Official DOM snapshot |
| Background navigation continuity | ChatGPT and Google Web AI | Completed, enabled, and device verified | Bounded official WebView recovery |

All production transports keep the official page authoritative. They do not export
cookies, credentials, request headers, or private conversation content outside the
device. Every observer is bounded and emits nothing on malformed or unsuccessful
responses.

The Google conversation directory persists the timestamp of the last successful
official directory response. Cached rows render immediately; a legacy or expired cache
is then refreshed in the background, while a recently verified cache does not trigger
another DOM read every time the side menu opens. The version 1 cache remains readable
and migrates as stale instead of being discarded or misreported as freshly verified.

Previously visited Google conversation bodies use a provider-scoped, URL-validated
snapshot before official navigation starts. The cache is bounded to 30 days, 128 files,
24 MiB total, and 2 MiB per conversation. A missing, expired, corrupt, or mismatched
snapshot renders an empty loading state and continues through the official page.

Google reply observation keeps fast bounded snapshots only until the matching assistant
reply appears. Once streaming is visible, MutationObserver and the passive same-origin
completion signal remain primary while dense polling is replaced by four sparse watchdogs
over 68 seconds. Repeated streaming snapshots cannot stack timers, completion cancels the
remaining watchdog immediately, and the official DOM snapshot remains the fallback.

ChatGPT streaming remains event-driven while the verified private response observer is
producing the current turn. Private progress schedules the native snapshot directly,
duplicate DOM mutations are ignored during that interval, and a four-second read-only
watchdog preserves official DOM reconciliation if private progress stalls. When no private
stream is available, the existing 400 ms bounded DOM heartbeat remains unchanged.

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
connection failure or leaving a large action card over the conversation. A bounded,
one-second read-only reconciliation tail then observes the official page for at most
two minutes without clicking hang-up again; a late official exit automatically closes
the orb and reuses the existing conversation refresh path.

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
