# ChatGPT Web Private Voice Native WebRTC Research

## Objective

Determine whether the signed-in ChatGPT web voice flow can keep its identity and
session bootstrap inside the persistent WebView while Android owns the media
transport through a native WebRTC stack. This is not the OpenAI developer API
route and does not use an API key.

## Production Boundary

Production remains native UI plus a persistent background WebView identity layer
plus the official page-created WebRTC session. The research build is opt-in through
`ELON_CHATGPT_PRIVATE_RESEARCH=true`. Failure or an unknown protocol shape must keep
the official page-created WebRTC fallback.

The observer must never export or persist Cookie values, request header values,
request or response bodies, SDP, ICE candidates, device labels, track identifiers,
temporary credentials, or private conversation content.

## Evidence Stages

1. Observe sanitized request families, path templates, status classes, response key
   categories, microphone grant state, peer-connection calls, and WebRTC state enums.
2. Confirm one complete official voice start and stop on a research APK without
   changing the default transport.
3. Classify whether the bootstrap is portable: required fields must be available in
   page memory, short-lived, scoped to the same user session, and separable from the
   official React voice state.
4. Only after stage 3 succeeds, add an in-memory handoff contract and a native WebRTC
   dependency. The contract must be single-use, bounded by expiry, and incapable of
   logging or durable storage.
5. Native takeover may become an experiment only after connect, duplex audio, stop,
   transcript reconciliation, background recovery, and official fallback all pass.

## Current Decision

Stage 2 is complete on Android research build `1.1.1318 (1328)`. One official
voice start produced this redacted sequence:

1. The page created a peer connection, data channel, local audio offer, and local
   description.
2. The page prepared the current conversation and short-lived sentinel state.
3. The page sent multipart `FormData` to `POST /realtime/wm`.
4. A successful response returned HTTP `201` with text shaped as a remote answer.
5. The page applied that answer, received a remote audio track, and reached ICE and
   peer-connection `connected` states.

This proves that native media ownership is structurally possible: Android can
create the offer and own the audio peer connection while a same-origin hidden
WebView remains the identity and bootstrap authority. Cookie export to an Android
HTTP client is neither required nor desirable.

Android research build `1.1.1320 (1330)` completed the bounded multipart check.
The request has an offer-like session-description field and a separate `session`
text field. The `session` field is JSON, not an opaque credential. Its redacted
top-level shape contains model selection, reasoning effort, chat mode, client
tools, and current-conversation binding. No field value crossed the bridge.

Stage 3 has therefore proved that the bootstrap body is portable inside the page
process. It has not yet proved that the page-owned session template can drive a
React-independent peer connection or that its lifetime permits fast re-entry.
Those are separate relay experiments and must not repeat protocol observation.

If that contract is stable, the next implementation is an in-memory single-use
relay:

- Android native WebRTC creates the audio offer.
- The hidden same-origin WebView performs the official conversation/sentinel
  preparation and builds the exact multipart request from page-owned state.
- The WebView returns only the remote answer to the native peer-connection owner;
  the handoff is never logged or persisted and expires immediately after use.
- Android owns microphone/audio routing, connection state, pause/resume, and stop.
- Transcript and current-conversation reconciliation remain bound to the official
  page session; any unknown shape, timeout, or mismatch falls back to the existing
  official page-created WebRTC route.

The repository intentionally has no Android native WebRTC dependency yet. The
official WebRTC Android source exposes `PeerConnectionFactory`, audio sources,
audio tracks, offers, answers, and peer observers, but the upstream project expects
Android consumers to build its JNI-backed SDK. A precompiled dependency must be
reviewed for provenance, ABI size, update cadence, and audio-only packaging before
it enters even a research APK.

The next capability is tracked separately as
`android_chatgpt_web_private_voice_native_relay_v1`. Its first proof must reuse the
captured page-local `session` template in memory, create an independent peer without
depending on the official React voice state, expire after one use, and fall back to
the existing official page-created WebRTC flow on any mismatch.
