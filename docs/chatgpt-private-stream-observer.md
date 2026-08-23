---
capability_id: android_chatgpt_private_stream_observer_v1
implementation_status: completed
verification_status: offline_verified_device_pending
production_default: false
repeat_implementation: not_required
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

The versioned parser and transport have deterministic coverage for fragmented
SSE frames, completion, conversation isolation, DOM handoff, rich-part
preservation, request single-execution, and request metadata non-access. The
existing DOM streaming and send-settle tests also pass.

Device evidence is still required for first-token latency, repeated sends,
background recovery, and a resource comparison. Until that one targeted pass
is complete, `ELON_CHATGPT_PRIVATE_STREAM_OBSERVER` defaults to `false`. The
implementation should not be rewritten or sent through broad exploratory
testing unless the observed endpoint or event contract changes.
