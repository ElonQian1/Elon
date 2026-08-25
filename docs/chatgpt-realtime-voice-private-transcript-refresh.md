---
capability_id: android_chatgpt_realtime_voice_private_transcript_refresh_v1
implementation_status: completed
verification_status: targeted_tests_passed_device_pending
production_default: true
repeat_research: not_required_without_regression
---

# ChatGPT realtime voice private transcript refresh

Realtime voice exit reuses the verified ChatGPT conversation-body transport instead
of waiting only for the official DOM to settle. The request remains inside the
signed-in ChatGPT WebView and is limited to the current `/c/{id}` path.

## Runtime contract

- Uses authenticated same-origin `GET /backend-api/conversations/{id}` only.
- Never exports cookies, copied request context, identifiers, titles, or message text.
- Never sends or replays a private POST and never navigates the official page.
- Coalesces concurrent refreshes for the same conversation into one in-flight request.
- Reuses the existing adaptive timeout, cooldown, and circuit breaker.
- Emits a native message snapshot only after a successful bounded parse.
- Keeps the retained native transcript visible and always requests the official DOM
  snapshot as the authoritative fallback.

This capability extends `android_chatgpt_private_conversation_prefetch_v1`; it does
not introduce another endpoint or a second health policy. Upstream response research
must not be repeated without a current regression.

## Verification

Passed offline:

- private transport syntax and contract tests;
- current-conversation path binding;
- same-conversation single-flight behavior;
- Android production voice wiring and recovery-gate unit tests.

Still pending user-supervised device evidence: enter realtime voice, speak, exit, and
confirm that the retained transcript remains visible while the updated turn arrives.
No audio or conversation text may be recorded in the evidence.
