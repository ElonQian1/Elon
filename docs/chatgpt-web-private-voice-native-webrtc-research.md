# ChatGPT Web Private Voice Native WebRTC Research

## Objective

Determine whether the signed-in ChatGPT web voice flow can keep its identity and
session bootstrap inside the persistent WebView while Android owns the media
transport through a native WebRTC stack. This is not the OpenAI developer API
route and does not use an API key.

## Production Boundary

Production now uses native UI plus a persistent background WebView identity/bootstrap
layer plus an Android-owned WebRTC media session. The dedicated
`ELON_CHATGPT_PRIVATE_VOICE_NATIVE_RTC` capability defaults to enabled and does not enable
the broader private-research probes. Failure or an unknown protocol shape keeps the
official page-created WebRTC fallback.

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
process. Later device work proved that the page-owned session template can drive a
React-independent peer connection, but also exposed a one-shot bootstrap race. The
media proof is complete and must not be repeated; atomic bootstrap ownership is the
remaining relay experiment.

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

The repository now compiles against the pinned research dependency
`io.github.webrtc-sdk:android-prefixed-stripped:144.7559.14`. Normal builds use it
as `compileOnly`; the JNI libraries are added with `runtimeOnly` only when both
`ELON_CHATGPT_PRIVATE_RESEARCH=true` and
`ELON_CHATGPT_PRIVATE_VOICE_NATIVE_RTC=true`. This keeps the experiment out of the
production APK while retaining a compile-time contract with the Android WebRTC API.

The next capability is tracked separately as
`android_chatgpt_web_private_voice_native_relay_v1`. Its first proof must reuse the
captured page-local `session` template in memory, create an independent peer without
depending on the official React voice state, expire after one use, and fall back to
the existing official page-created WebRTC flow on any mismatch.

## Stage 4 Relay Contract

The same-origin single-use relay contract is now implemented behind
`ELON_CHATGPT_PRIVATE_RESEARCH`. It installs at document start before the bounded
observer and captures only a valid `/realtime/wm` multipart template in page memory.
Android may submit a replacement audio offer through a versioned page function and
poll once for the answer. The relay:

- accepts one bounded audio SDP offer and one in-flight exchange;
- reuses the exact page-owned request URL, request shape, `session` value, and
  runtime headers without exporting them from the page;
- removes the multipart `Content-Type` so the browser creates a new boundary;
- uses a 15-second upstream timeout, a two-minute template lifetime, a 30-second
  result lifetime, and a four-result bound;
- deletes the request template values after the exchange and consumes results when
  Android reads them;
- exposes only structural availability and fixed safe failure codes; and
- preserves the official page-created WebRTC route as the only production default.

Node and Kotlin contract tests prove template replacement, single-use behavior,
bounded polling, SDP validation, safe error collapse, and redacted answer string
rendering. This is not yet a device proof of React-independent media. The next stage
must add the research-only native audio peer, pass its offer through this relay, apply
the answer, and verify remote audio plus clean close on one supervised device before
the capability can be considered for production.

## Stage 5 Native Peer Implementation

The research-only native peer is implemented and a signed research Release APK has
compiled with all four packaged WebRTC JNI ABIs. The native path now:

- initializes one process-wide Android `PeerConnectionFactory`;
- creates an audio-only Unified Plan peer connection, native microphone track, and
  the bounded data-channel shape observed from the official page;
- creates and applies the local offer, sends it through the page-local relay, and
  applies the returned answer without logging SDP, ICE, credentials, or content;
- exposes only structural connection, remote-audio, data-channel, mute, timeout,
  and close state to the research control surface; and
- closes on a 20-second connect timeout and leaves the official page-created WebRTC
  path available as the production fallback.

## Stage 5 Device Evidence

Supervised Xiaomi research builds through `1.1.1328 (1338)` proved the native
media path. The native peer reached connected state, received remote audio, opened
the observed empty-label data channel, and kept Android microphone capture active.
Android audio routing was moved to communication mode and explicitly selected the
built-in speaker when no headset was connected.

The user then heard two copies of the same answer. This is positive duplex evidence:
both the page-created official peer and the Android-owned peer were carrying audio.
It is also evidence that closing only the currently observed official peer is not a
complete handoff because the page may create a replacement peer. A page-level
takeover lock now disables late sender, receiver, transceiver, and replacement-track
audio while native ownership is active, and restores official media after a native
startup failure.

The bootstrap-ownership blocker has since been closed. The relay arms the native offer
before the official request, atomically replaces the first page-owned SDP field, and sends
one same-origin upstream request. The official peer is suspended only after takeover;
native failure releases the lock and uses the untouched page-created path. A supervised
device run verified one native audio answer, an open data channel, and no duplicate audio.
The managed path is therefore the production default. Live transcript event-envelope
acceptance remains tracked separately and must not be inferred from the audio proof.
