---
capability_id: android_chatgpt_private_stream_observer_v1
implementation_status: websocket_compact_envelope_adapter_device_pending
verification_status: device_observed_websocket_envelope
production_default: false
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

Research APK `1.1.1245 (1255)` contains that final `update_content` decoder and
passed the release build plus deterministic single-request, bounded-buffer,
envelope, completion, and fallback tests. Device installation was deferred
because both USB and the remembered wireless ADB address went offline. Keep the
production flag off until one controlled response verifies canonical assistant
text from this decoder; after that single verification, record the APK and
adapter evidence here and enable the guarded default rather than repeating
protocol discovery.
