---
capability_id: android_chatgpt_private_send_dispatch_observer_v1
implementation_status: completed
verification_status: device_structural_verified
production_default: true
repeat_research: not_required_without_regression
---

# ChatGPT private send dispatch observer

This capability shortens native send confirmation without creating a second
prompt request. The official ChatGPT page remains the only sender and continues
to own React state, conversation ancestry, cookies, headers, retries, stop
controls, and recovery.

## Behavior

- The page-local observer recognizes only same-origin `POST` requests to the
  current ChatGPT conversation endpoint.
- It records only an in-memory sequence number, bounded page path, and
  observation time. It never reads or exports request headers, cookies, body,
  prompt text, conversation identifiers, or response content.
- A native send command is acknowledged as soon as the official page dispatches
  its request. If that evidence is unavailable, the existing composer, stream,
  and DOM acceptance checks remain authoritative.
- Requests tagged as an Elon private transport are ignored so a future
  experiment cannot confirm itself or cause duplicate submission.

## Production decision

`android_chatgpt_private_send_dispatch_observer_v1` is enabled with the already
verified private stream observer. It improves acknowledgement latency while
preserving the official page's single-send semantics and fallback behavior.

A direct native conversation POST is deliberately not enabled. It would update
the server without synchronously updating the official page's React state and
conversation parent pointers, which can make the official fallback stale or
duplicate a later message. That path requires an upstream-supported state
handoff contract before it can replace the official sender.

## Evidence

Deterministic tests cover endpoint and origin filtering, GET and cross-origin
rejection, synchronous request failure, private-transport self-confirmation,
conversation-path isolation, request-content non-access, asset order, and the
adapter fast-ack path. Existing send-settle and private stream tests continue to
pass, along with targeted Android unit tests and a release build.

On Xiaomi `e0d909c3`, adapter `178` preserved the signed-in session and the
official send entered native streaming with the message count advancing. The
controlled assistant completion did not finish inside the acceptance window,
so this evidence verifies dispatch and native state integration, not upstream
reply latency. The synthetic generation was stopped by force-stopping only the
APK process; app data, WebView cookies, and login state were not cleared.
