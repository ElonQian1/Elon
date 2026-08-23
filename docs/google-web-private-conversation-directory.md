---
capability_id: android_google_web_private_conversation_directory_v1
implementation_status: completed
verification_status: device_verified
production_default: true
repeat_implementation: prohibited_without_regression_evidence
---

# Google Web private conversation directory

This capability keeps the native Google Web AI conversation list warm without opening
the official sidebar or polling visible DOM rows.

## Boundary

- A document-start observer passively reads successful same-origin
  `GET /httpservice/web/AimThreadsService/ListThreads` XHR responses. It never creates,
  replays, or modifies a request and never reads request headers, request bodies, cookies,
  or account tokens.
- The parser accepts at most 200 bounded identifier/title rows. It derives official
  restorable URLs from the active official `csuir` template and sends them through the
  native Google URL allowlist before persisting them in the existing AtomicFile cache.
- Cached rows render immediately. Missing, malformed, cross-origin, or unmappable
  responses emit nothing, so the existing local observation and official WebView paths
  remain the automatic fallback.
- Research-only network shape logging remains disabled in production builds.

## Verification

Adapter version 37 passed focused JavaScript contracts and Android Release unit tests.
A signed research build was installed with `adb install -r` without clearing app data or
cookies. The native Google Web AI surface imported the official directory, opened a
controlled matching conversation from the native navigation list, and restored the
original conversation. The acceptance report exposed only counts and booleans; it did
not emit conversation content.
