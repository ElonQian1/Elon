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
| Streaming reply observer | ChatGPT | Completed and enabled | Official DOM snapshot |
| Realtime voice transcript refresh | ChatGPT | Completed and enabled; supervised voice exit pending | Retained native transcript and official DOM snapshot |
| Realtime voice background continuity | ChatGPT | Completed and enabled; native/system overlay handoff and media auto-pause device verified, manual overlay actions pending | Official WebView voice and foreground notification |
| Conversation directory | Google Web AI | Completed and enabled | Local cache and official page |
| Reply completion observer | Google Web AI | Completed and enabled | Official DOM snapshot |
| Background navigation continuity | ChatGPT and Google Web AI | Completed, enabled, and device verified | Bounded official WebView recovery |

All production transports keep the official page authoritative. They do not export
cookies, credentials, request headers, or private conversation content outside the
device. Every observer is bounded and emits nothing on malformed or unsuccessful
responses.

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
connection failure or leaving a large action card over the conversation.

## Audited non-capabilities

- Google direct send is not implemented. The official form/navigation path already
  confirms dispatch from navigation, composer, query, or streaming state; a second
  request could split official page state from server state.
- Google conversation-body prefetch is not implemented because controlled endpoint
  inventory has not identified a safe same-origin detail endpoint. Existing per-thread
  snapshots remain stale-while-revalidate cache, followed by official navigation.
- ChatGPT realtime voice does not persist or replay live WebRTC credentials. The APK
  caches the loaded WebView session and per-conversation launch hints, then lets the
  official page create a fresh connection for each start.
- Redacted research probes remain disabled in production. They are not user features
  and require a new compatibility question before they may be re-enabled.

## Delivery rule

Completed entries are not reimplemented without regression evidence. A deferred or
audited entry may change only after a new controlled observation proves a safer and
faster official same-origin path with automatic fallback.
