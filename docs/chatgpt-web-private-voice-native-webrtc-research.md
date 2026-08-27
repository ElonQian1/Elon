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

The repository has no Android native WebRTC dependency. Adding one before observing
the real web bootstrap would guess the private protocol and increase APK size and
maintenance cost without proving that the temporary session can be transferred.
The first implementation is therefore a redacted document-start observer with a
typed native parser. It records protocol shape, not credentials.
