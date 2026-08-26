---
capability_id: android_google_web_private_reply_observer_v1
implementation_status: completed
verification_status: device_verified_stream_to_completion_v1_1_1303
production_default: true
repeat_implementation: prohibited_without_regression_evidence
---

# Google Web private reply observer

This capability accelerates Google Web AI replies on the native chat surface while the
official page remains authoritative.

## Boundary

- It observes the official same-origin `GET /async/folif` response only as a completion
  signal. It does not create, replay, or modify the request.
- It does not read response bodies, request headers, request bodies, cookies, account
  tokens, or unrelated page content.
- After the completion signal, it collects only newly inserted, visible answer leaf
  nodes that appear after the controlled user prompt in the official main content.
- If no safe candidate is found, it emits nothing. The existing DOM extractor and
  snapshot refresh coordinator continue as the automatic fallback.
- Research-only endpoint diagnostics remain disabled in production builds.

## Verification

Release adapter version 36 passed the focused JavaScript contracts and Android Release
build. A signed research build was installed with `adb install -r` without clearing app
data or cookies. One controlled exact-marker message completed in the native Google Web
AI chat, the original conversation was restored, and structural diagnostics showed the
official completion signal followed by one safe new answer candidate and one completed
reply. No private conversation content was emitted by the acceptance script.

Release adapter version 37 retained that observer and replaced dense reply polling after
the first streaming snapshot with four sparse watchdogs. APK `v1.1.1303 (1313)` passed a
second isolated exact-marker acceptance: native state observed streaming, then a completed
ready reply, and the original conversation was restored. The acceptance output contained
only provider/version flags, state-transition booleans, counts, and recovery status.
