---
capability_id: android_chatgpt_private_conversation_prefetch_v1
implementation_status: completed
verification_status: device_verified
production_default: true
repeat_research: not_required
---

# ChatGPT private transport research

This document records non-sensitive Android research evidence. The official
WebView/DOM path remains canonical and is always available as fallback.

## Build gates

- `ELON_CHATGPT_PRIVATE_RESEARCH` enables page-local endpoint observation.
- `ELON_CHATGPT_PRIVATE_CONVERSATION_PREFETCH` controls the production
  conversation snapshot request. It defaults to `true`; an emergency build can
  explicitly set it to `false` without changing source.
- Conversation prefetch is independent from the research observer. Normal
  builds keep the observer disabled while the verified transport stays active.

The page keeps request headers only in page memory. Session storage contains
only bounded health metadata: latency estimates, success/failure counts,
cooldown deadline, and the last outcome. It contains no cookie, authorization
header, conversation identifier, title, or message content.

## 2026-08-23 device evidence

Device: Xiaomi `e0d909c3`, existing signed-in WebView state preserved.

- The current full conversation endpoint observed from the official page is
  `GET /backend-api/conversations/{id}`.
- The response can expose modern root-level `messages` arrays instead of the
  older `mapping` tree. The research parser supports both forms.
- A warm six-switch sample produced four private requests. All four reached
  HTTP 200 in 652-871 ms. Three produced usable snapshots; one crossed the old
  900 ms total response-plus-parse budget and was classified as a timeout.
- Successful native-command-to-private-outcome times were 830-992 ms. The
  corresponding full official navigation commonly completed in 2.2-2.4 s.
- No prompt was sent, no private content was emitted, and no cookie or app data
  was cleared during these samples.

The adaptive policy now allows at most 1000 ms before the first verified
success, then 350-1200 ms based on observed latency. Timeout, auth, parse, empty,
and network failures enter bounded cooldowns before the official fallback is
tried again.

## Production decision

Conversation snapshot prefetch is a completed production capability and is
enabled by default. The verified path improves warm conversation display while
the existing bounded timeout, cooldown, and official navigation fallback keep
the original behavior available on every failure. This capability should not be
reimplemented or sent back through exploratory research unless the upstream
response contract changes or current regression evidence identifies a defect.

The research transport is a replaceable provider component. Sending, streaming,
voice, and directory replacement require separate evidence and must not inherit
this read-path result automatically.
