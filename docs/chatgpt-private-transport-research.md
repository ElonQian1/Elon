# ChatGPT private transport research

This document records non-sensitive Android research evidence. The official
WebView/DOM path remains canonical and is always available as fallback.

## Build gates

- `ELON_CHATGPT_PRIVATE_RESEARCH` enables page-local endpoint observation.
- `ELON_CHATGPT_PRIVATE_CONVERSATION_PREFETCH` additionally enables the
  experimental conversation snapshot request.
- Conversation prefetch cannot be enabled unless the research gate is enabled.
- Both flags default to `false` in normal builds.

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

## Default decision

Conversation prefetch remains disabled by default. A small sample shows a real
latency opportunity, but not enough reliability for production rollout. Do not
make it default until a supervised, content-free sample meets all of these:

- at least 30 eligible switches across cold start, warm state, and app resume;
- at least 95 percent usable snapshots;
- private snapshot p90 no slower than 1000 ms;
- failed attempts add no more than the configured budget before fallback;
- a 15-minute alternating WebView/private run shows lower or equal thermal and
  battery impact without login, history, project, attachment, or voice regressions.

The research transport is a replaceable provider component. Sending, streaming,
voice, and directory replacement require separate evidence and must not inherit
this read-path result automatically.
