---
capability_id: android_chatgpt_realtime_voice_data_channel_transcript_v1
implementation_status: completed
verification_status: targeted_tests_passed_device_event_shape_pending
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
- Requires a bounded event type and stable item or response identifier.
- Limits each data-channel message to 256 KiB and each transcript stream to 64 KiB.
- Deduplicates bounded event identifiers and keeps live text in memory only.
- Never logs or publishes transcript text, payloads, cookies, SDP, ICE, credentials, or
  request headers.
- Exposes only structural data-channel and parsed-transcript event counts through MCP.
- Uses the documented `oai-events` label on cold start, while a bounded label observed
  from the current official page overrides that preset.
- Accepts a server-created channel only while the local channel is still connecting; an
  already-open local channel remains authoritative so duplicate streams are not attached.
- Leaves the WebView as the identity and official session owner.
- Reconciles native preview bubbles with the existing same-origin conversation refresh
  and official DOM snapshot after the voice turn settles.

Malformed, unknown, missing, or changed events produce no capability error and do not
interrupt audio. The final conversation refresh and official DOM remain authoritative.

## Verification

Passed offline:

- current and legacy assistant transcript event parsing;
- user transcription delta/completed event parsing;
- malformed, unbound, unrelated, and oversized payload rejection;
- incremental native bubble accumulation and duplicate-event suppression;
- replacement by the authoritative conversation snapshot;
- structural-only MCP state reporting.

Still pending device evidence: complete one native WebRTC voice turn and confirm that the
data-channel message count and parsed transcript count advance and that native bubbles
appear before final conversation reconciliation. Evidence must not contain transcript
text or raw event payloads.
