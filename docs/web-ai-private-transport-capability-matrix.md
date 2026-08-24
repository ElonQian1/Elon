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
| Conversation directory | Google Web AI | Completed and enabled | Local cache and official page |
| Reply completion observer | Google Web AI | Completed and enabled | Official DOM snapshot |

All production transports keep the official page authoritative. They do not export
cookies, credentials, request headers, or private conversation content outside the
device. Every observer is bounded and emits nothing on malformed or unsuccessful
responses.

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
