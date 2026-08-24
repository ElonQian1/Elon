---
capability_id: android_chatgpt_private_conversation_project_directory_v1
implementation_status: completed
verification_status: device_verified
production_default: true
repeat_implementation: prohibited_without_regression_evidence
---

# ChatGPT private conversation and project directory

This capability keeps the native ChatGPT conversation and project directory warm
without routinely opening, scrolling, or navigating the official sidebar.

## Boundary

- A document-start observer passively clones successful same-origin official `GET`
  responses for the bounded conversation list, project sidebar, and per-project
  conversation families. It never creates, replays, delays, or modifies a request.
- The observer does not read request headers, request bodies, cookies, account tokens,
  or response headers. It emits only the conversation and project identifiers, titles,
  paths, and bounded presentation metadata already required by the native directory.
- Responses are limited to 1 MiB, 200 conversations, and 40 projects. Malformed,
  oversized, failed, non-GET, and cross-origin responses emit nothing.
- Existing AtomicFile directory persistence renders cached rows immediately. Passive
  official responses refresh that cache asynchronously with source
  `official_private`. The official WebView and DOM collector remain the automatic
  fallback for missing responses, unsupported page versions, and explicit refreshes.
- Direct private message POST remains disabled. Official page actions continue to own
  React conversation state, parent pointers, authentication, verification, and sends.

## Verification

Focused JavaScript contracts proved single execution of the official request, bounded
global/project parsing, XHR and fetch observation, and rejection of POST, cross-origin,
invalid, and malformed data. Existing ChatGPT private transport, stream, send-observer,
Google directory contracts, and Android Release tests remained green.

The production APK `v1.1.1282 (build 1292)`, source `cd6f73b9c`, and SHA-256
`a0bd43043f4c3b4d4e3e061aa55a4f3e71ae3407cef89aad237d1b670b596d13` were published
and installed with `adb install -r` on the authorized Xiaomi device without clearing
app data or WebView cookies. Read-only acceptance observed adapter 179, ready bridge
and composer, 126 cached conversations, 5 projects, and a current bounded 100-row
directory source of `official_private`. No message was sent and no private content was
emitted.

## Completion rule

Reuse this capability for ordinary directory loads. Reopen implementation research
only when a current production regression shows that official response shapes are no
longer accepted or the automatic DOM fallback fails.
