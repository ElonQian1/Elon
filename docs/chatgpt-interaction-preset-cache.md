---
capability_id: android_chatgpt_interaction_preset_cache_v1
status: completed
production_default: true
verification: device_verified_v1_1_1367
---

# ChatGPT interaction preset cache

## Contract

- Model and tool menus render built-in presentation presets or the last bounded user-scoped
  snapshot immediately. Official discovery refreshes them asynchronously.
- A cached option never authorizes an old DOM identifier. Selection resolves the current
  official semantic ID before invoking the page action.
- Temporary chat remains visible and actionable while the official control is unobserved.
  The APK queues one desired state, emits at most one accepted mutation, and reports success
  only after the live control confirms the selected state.
- A new-conversation request is accepted while the background session is `idle` or `loading`.
  Native presentation changes immediately; the current official bridge receives exactly one
  navigation command when ready. Duplicate taps are rejected until the boundary settles.
- Login-required, terminal-error, unsupported-adapter, and unconfirmed states continue to
  fail closed or use the current official WebView. No write is replayed automatically.

## Cache bounds

Stable model, tool, and feature catalogs retain the existing six-hour freshness window,
30-day maximum retention, user isolation, and bounded persisted snapshot. Contextual controls
remain page-scoped and memory-only because their selected state and DOM identifier are live
session facts.

## Verification

- `WebChatProductionInteractionCacheTest`
- `WebChatProductionInteractionSnapshotCodecTest`
- `WebChatProductionCapabilityPrewarmerTest`
- `WebChatProductionHeaderActionsTest`
- `WebChatTemporaryChatIntentQueueTest`
- `ChatGptSessionNavigationActionsTest`
- `scripts/test-chatgpt-web-new-conversation.js`
- `scripts/test-chatgpt-web-temporary-chat-adapter.js`

Research APK `v1.1.1367 (1388)` was installed with `adb install -r` over the existing
application without clearing data. A production friend-chat request made while the session
reported `loading` moved from an existing conversation to a blank, composer-ready official
conversation and then restored the origin. A separate isolated blank conversation verified
that temporary chat was actionable, changed the official selected state, restored that state,
and reopened the original conversation. Acceptance output contained structural booleans only.

## Privacy boundary

The capability does not export cookies, credentials, request headers, DOM text, conversation
content, or control identifiers. The persistent WebView remains the identity and official
state owner; native presets are presentation and intent only.
