---
capability_id: android_web_ai_background_navigation_continuity_v1
implementation_status: completed
verification_status: device_structural_verified
production_default: enabled
reviewed_at: 2026-08-25
---

# Web AI background navigation continuity

## User-visible behavior

Switching between ChatGPT and Google Web AI no longer cancels a Google page that is
already loading. Returning to either provider keeps the cached native conversation
visible while the APK reattaches its versioned page adapter to the existing document.

## Recovery policy

The shared resume policy distinguishes four cases:

1. An initial load deferred before it started is retried once through the existing
   recovery budget.
2. A supported page that finished while hidden is repaired and snapshotted before a
   reload is considered.
3. A supported page still loading keeps its current navigation and only restarts the
   bounded stall watchdog.
4. A failed or stalled page falls back to the existing official WebView reload path.

No private POST, credential replay, Cookie export, background polling loop, or live
WebRTC reuse is introduced. The official page remains authoritative.

## Evidence

- Shared policy unit tests cover deferred, unsupported, failed, finished, and in-flight
  states.
- Provider contract tests require both background sessions to use the shared policy
  and prevent Google from stopping a page during provider switches.
- Xiaomi device acceptance preserved an in-flight Google navigation across a rapid
  Google -> ChatGPT -> Google switch. ChatGPT stayed ready, Google returned to ready
  with adapter version 37 and its cached directory count unchanged in about 1.77 s,
  and the original ChatGPT provider was restored afterward. No prompt was sent and no
  conversation content was recorded.
