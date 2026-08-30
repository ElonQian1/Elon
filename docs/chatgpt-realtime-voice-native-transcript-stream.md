---
capability_id: android_chatgpt_realtime_voice_data_channel_transcript_v1
implementation_status: completed
verification_status: device_supervised_native_streaming_bubbles_verified
production_default: true
repeat_research: not_required_without_regression
---

# ChatGPT realtime voice native transcript stream

The production native realtime peer already owns the audio WebRTC connection and its
official data channel. This capability consumes only bounded transcript events from that
channel and presents them in the existing native conversation UI while audio continues.
It does not read captions from the voice-page DOM and does not use the paid OpenAI
Realtime API.

## Runtime contract

- Accepts allowlisted user and assistant transcript delta/final event types only.
- Reconstructs ChatGPT Web's compact `chat_message_delta` state with bounded channel,
  path, patch, collection, and result limits before extracting message text.
- Bounded UTF-8 JSON is parsed whether WebRTC marks the frame as text or binary.
- Requires a bounded event type and stable item or response identifier.
- Limits each data-channel message to 256 KiB and each transcript stream to 64 KiB.
- Deduplicates bounded event identifiers and keeps live text in memory only.
- Never logs or publishes transcript text, payloads, cookies, SDP, ICE, credentials, or
  request headers.
- Exposes only structural data-channel and parsed-transcript event counts through MCP.
- Uses the empty-label channel shape observed from ChatGPT Web on cold start. It must not
  borrow the public Realtime API's `oai-events` example because this transport targets the
  signed-in ChatGPT website protocol. A bounded label observed from the current official
  page still overrides the preset.
- Retains bounded local and server-created channels until close, consumes transcript events
  from any open channel, and relies on the existing event-id continuity layer to suppress
  duplicate transcript updates.
- Leaves the WebView as the identity and official session owner.
- Reconciles native preview bubbles with the existing same-origin conversation refresh
  while the voice turn is active and after it settles. When no parsed data-channel event
  has arrived, the current conversation refresh runs about every 1.5 seconds. Once live
  events arrive, private reconciliation becomes sparse (about every 6 seconds), with an
  official DOM watchdog about every 12 seconds.
- Starts the existing transcript-continuity owner from the production managed voice entry,
  so authoritative snapshots can update native bubbles without an empty DOM clearing the
  retained conversation.

Malformed, unknown, missing, or changed events produce no capability error and do not
interrupt audio. The same-origin current-conversation refresh and sparse official DOM
watchdog remain authoritative. They reuse the existing single-flight, timeout, cooldown,
and circuit-breaker policy rather than introducing a second transport.

## Verification

Passed offline:

- compact private message delta add, append, replace, patch, remove, and truncate;
- current and legacy assistant transcript event parsing;
- user transcription delta/completed event parsing;
- malformed, unbound, unrelated, and oversized payload rejection;
- incremental native bubble accumulation and duplicate-event suppression;
- text and binary UTF-8 data-channel frame handling;
- managed production-entry transcript initialization;
- event-first active refresh cadence with private and DOM fallback throttling;
- replacement by the authoritative conversation snapshot;
- structural-only MCP state reporting.

Passed on device without retaining private content or raw payloads:

- the official voice page emitted the private `chat_message_delta` event shape during a
  fixed synthetic diagnostic turn;
- the native WebRTC peer connected with remote audio and the official data channel open;
- the production-default Release build completed after the private delta decoder was wired
  into the native peer.
- release `v1.1.1346 (build 1367)`, sourced from `145819bb2`, started a supervised native
  voice turn from an empty new conversation with the native peer connected, remote audio
  active, and the data channel open;
- structural observation counted 263 bounded data-channel frames, 227 parsed transcript
  events, and seven native conversation bubbles (three user and four assistant) without
  reading or retaining their text;
- the user confirmed that the native bubbles appeared incrementally while the voice turn
  was still active, and the call then closed with remote audio and the data channel stopped.

The supervised presentation gap is closed. Repeat protocol research or a larger sample is not
required unless a current regression shows missing native bubbles, duplicate reconciliation,
or a changed official event shape.
