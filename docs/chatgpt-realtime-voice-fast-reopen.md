---
capability_id: android_chatgpt_realtime_voice_fast_reopen_v1
implementation_status: completed
verification_status: targeted_tests_passed_device_pending
production_default: true
repeat_research: not_required_without_regression
---

# ChatGPT realtime voice fast reopen

This capability shortens repeated realtime voice starts while preserving the
official ChatGPT page as the owner of the audio session.

## Behavior

- A ready conversation reuses its loaded WebView session and cached voice-entry
  capability instead of forcing session recovery.
- The preparation command still protects native and official drafts. When it
  confirms that no page mutation is needed and the voice control is already
  current, voice starts immediately without another control refresh delay.
- Retrying a failed start follows the same direct, refresh-controls, or
  recover-session plan. It no longer reloads a healthy page unconditionally.
- Tapping the voice entry after a native failure retries the operation instead
  of only raising the unchanged failed surface.

## Boundary

The app does not persist or reuse a live WebRTC connection. ChatGPT creates a
new official voice connection for each start, so conversation attribution and
provider security state remain authoritative. The existing official-page
fallback remains available.

## Verification

Targeted Android tests cover the direct no-refresh path, failed-start retry,
session recovery selection, prepared-draft behavior, close/reopen behavior,
and the persisted capability hint. A supervised device pass is still required
to measure first and repeated connection time with microphone use.
