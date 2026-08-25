---
capability_id: android_chatgpt_private_stream_observer_v1
implementation_status: completed
verification_status: device_verified_protocol_adaptive_watchdog_regression_pending
production_default: true
repeat_implementation: legacy_sse_and_websocket_discovery_not_required
---

# ChatGPT private stream observer

This capability observes a clone of the official page's same-origin streaming
response. The official page still owns prompt submission, request headers,
cookies, response consumption, stop controls, and recovery.

## Safety boundary

- It never creates or replays a conversation POST.
- It never reads, stores, or emits request headers or request bodies.
- It accepts only `POST /backend-api/f/conversation` responses whose content
  type is `text/event-stream`.
- Parsing, stream, path, or merge failures leave the existing DOM snapshot path
  active without changing the official response.
- The response clone and parsed text remain inside the ChatGPT page process.
  Native receives the same bounded message snapshot shape already used by the
  existing adapter.

## Current evidence

The versioned parser and transport have deterministic coverage for the legacy
message-shaped SSE contract, completion, conversation isolation, DOM handoff,
rich-part preservation, request single-execution, and request metadata
non-access. The existing DOM streaming and send-settle tests also pass.

The Xiaomi `e0d909c3` device pass used APK `1.1.1237 (1247)`, adapter `166`,
SHA-256 `5F9C0B0FFBD3C7E57A45D01886A7B01CC9D2F8B65366944E13BB85721150BB93`.
The production native chat received an assistant response about 10.7 seconds
after the probe began and restored the previous conversation. The official DOM
path therefore remained functional.

The cloned private response did not use the tested message-shaped SSE contract.
Its structural-only observations included `resume_conversation_token`,
`input_message`, `conversation_async_status`, and compact `c/o/p/v` fields. The
observer finished as `empty`, with zero accepted text frames after about 3.1
seconds. No prompt, answer, cookie, request header, or request body was emitted.

`ELON_CHATGPT_PRIVATE_STREAM_OBSERVER` must remain `false` until a bounded,
tested compact-protocol decoder can produce the same canonical assistant
message without replaying the request. Do not repeat the legacy SSE parser
experiment; the next work item is specifically compact-protocol decoding and
DOM equivalence testing.

## WebSocket envelope evidence

Adapter `168` installs a document-start, same-origin WebSocket tap before the
official page creates its connection. The tap observes only
`wss://ws.chatgpt.com` and `wss://chatgpt.com`, keeps at most 24 frames and 256
KiB in page memory, never changes outbound calls, and does not persist data.
The existing fetch observer remains available for older page contracts.

Xiaomi research APKs `1.1.1242 (1252)` through `1.1.1244 (1254)` confirmed the
official socket and its structural envelopes. Assistant delivery uses a
`conversation-update` frame with `payload`, then `conversation_id`,
`update_type`, and `update_content`. The native DOM fallback continued to show
the controlled assistant response, while no private text, cookie, header value,
or request body was emitted. The tested parser now handles bounded JSON/SSE
strings inside `reply`, `payload`, and `update_content` arrays without replaying
the request.

Research APK `1.1.1248 (1258)`, adapter `168`, established the production path
on Xiaomi `e0d909c3`. The official page sent the controlled prompt once and the
native observer received both `private_stream first` and `private_stream
success` from the page's existing `GET /backend-api/conversation/{id}/stream_status`
response before DOM completion. The native message snapshot contained the
expected controlled marker; no cookie, header value, request body, prompt, or
unrelated conversation content was emitted by the diagnostic evidence.

The completed transport observes only clones of official responses. It supports
legacy SSE, bounded official WebSocket envelopes, and the current JSON stream
status response. It never creates another request or replays a prompt; parse,
timeout, protocol, or merge failures leave the DOM path authoritative. The
production default is now enabled, and adapter `169` invalidates older page
generations. Do not repeat protocol discovery without a current regression.

## Adaptive native snapshot scheduling

Adapter `181` keeps the completed private observer as the primary streaming signal.
Each accepted private update schedules the native snapshot directly. While that signal is
active, duplicate DOM mutations no longer trigger another snapshot and the fixed 400 ms
heartbeat becomes a four-second read-only watchdog. If no private stream is available, the
original DOM heartbeat is unchanged; completion, parsing failure, timeout, or protocol drift
therefore continues through the official DOM fallback. Targeted policy and Android contract
tests cover watchdog selection, timer replacement, and adapter wiring. Device regression is
pending and does not invalidate the earlier private-stream protocol verification.
