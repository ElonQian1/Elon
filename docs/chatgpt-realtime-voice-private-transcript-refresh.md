---
capability_id: android_chatgpt_realtime_voice_private_transcript_refresh_v1
implementation_status: completed
verification_status: targeted_tests_passed_device_pending
production_default: true
repeat_research: not_required_without_regression
---

# ChatGPT realtime voice private transcript refresh

Realtime voice reuses the verified ChatGPT conversation-body transport while the call is
active and when it exits, instead of waiting only for the official DOM to settle. The
request remains inside the signed-in ChatGPT WebView and is limited to the current
`/c/{id}` path.

## Runtime contract

- Uses authenticated same-origin `GET /backend-api/conversations/{id}` only.
- Never exports cookies, copied request context, identifiers, titles, or message text.
- Never sends or replays a private POST and never navigates the official page.
- Coalesces concurrent refreshes for the same conversation into one in-flight request.
- Reuses the existing adaptive timeout, cooldown, and circuit breaker.
- Prefers native data-channel transcript events. Until an event is parsed, it refreshes
  the current conversation about every 1.5 seconds; after live events begin, it reconciles
  about every 6 seconds.
- Emits a native message snapshot only after a successful bounded parse.
- Keeps the retained native transcript visible while authoritative snapshots update the
  existing bubbles.
- Uses an official DOM snapshot only as a sparse watchdog (about every 12 seconds) and at
  exit, rather than continuously polling the page.

This capability extends `android_chatgpt_private_conversation_prefetch_v1`; it does
not introduce another endpoint or a second health policy. Upstream response research
must not be repeated without a current regression.

## Verification

Passed offline:

- private transport syntax and contract tests;
- current-conversation path binding;
- same-conversation single-flight behavior;
- event-first active refresh cadence and sparse DOM watchdog wiring;
- managed production-entry transcript-continuity initialization;
- Android production voice wiring and recovery-gate unit tests.

Still pending user-supervised device evidence: enter realtime voice, speak, exit, and
confirm that the retained transcript remains visible while the updated turn arrives.
No audio or conversation text may be recorded in the evidence.
