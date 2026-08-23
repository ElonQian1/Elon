---
capability_id: android_chatgpt_conversation_project_directory_cache_v1
implementation_status: completed
verification_status: device_verified
production_default: true
repeat_research: not_required
---

# ChatGPT conversation and project directory cache

This capability makes the native Android conversation sidebar cache-first.
It is separate from private conversation snapshot prefetch and does not call a
private transport.

## Production behavior

- Conversation rows, projects, and per-project conversation rows are restored
  from a bounded `AtomicFile` cache before the official directory refresh.
- The native sidebar renders restored rows immediately. A fresh cache does not
  start another automatic refresh.
- Expired or explicitly stale data refreshes in the background. An in-flight
  request and a recent failed request are protected by a cooldown.
- Official refresh results are merged by canonical conversation and project
  identity. A failed refresh keeps the cached rows visible.
- Per-conversation message snapshots use a separate bounded cache, so opening a
  cached directory row can preserve the previous native content while the
  official page navigates.
- Cookie, authorization headers, request headers, conversation text, and
  project titles are not written to logs or capability metadata.

The official WebView/DOM directory remains canonical and continues to update
the cache. Cache decode, expiry, corruption, or refresh failure falls back to
the official path without declaring that the upstream capability is absent.

## Verification

Deterministic Android tests cover cache codec validation, project-scoped merge,
cache-first rendering, expiry, request cooldown, and cached conversation
navigation.

On 2026-08-23, APK `1.1.1238 (1248)` with adapter `166` was force-stopped and
reopened on an authorized Xiaomi device without clearing application data or
WebView cookies. After selecting the production ChatGPT chat surface, the
native state reported a ready, authenticated adapter and restored non-empty
conversation and project directories in about 0.3 seconds. No message was sent
and no private title or conversation content was emitted.

## Decision

The directory cache is a completed production capability and is enabled by
design rather than by an experiment flag. Do not add another directory cache or
repeat broad research unless current regression evidence shows stale identity,
incorrect project membership, lost rows, or failure to fall back to the
official directory.
