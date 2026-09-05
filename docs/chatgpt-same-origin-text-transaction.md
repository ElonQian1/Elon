---
capability_id: android_chatgpt_same_origin_text_transaction_v1
implementation_status: completed
verification_status: device_verified_v1_1_1539_adapter_260
production_default: true
direct_private_post_status: not_device_verified_current_contract
repeat_implementation: forbidden_without_current_regression_evidence
---

# ChatGPT same-origin text transaction

This capability gives native ChatGPT text actions one versioned transaction owner for
send, stop, regenerate, stream completion, and official-page reconciliation. The existing
background WebView remains the identity and authoritative page runtime.

## Transport boundary

- Pure-text sends are single-flight and use stable bounded request IDs.
- A same-origin direct request is eligible only when the captured official request template
  is current, structurally pure text, route-bound, stream-confirmed, and free of one-time
  dynamic proof material.
- Current ChatGPT Web requests contain one-time dynamic proof. The runtime detects that
  before dispatch and immediately uses the official page transaction instead of cloning a
  request that would fail or wait for a long network timeout.
- Unknown completion never causes an automatic write replay. Read-only page reconciliation
  decides whether the native command settled.
- Response headers have a 15-second deadline. Accepted streams replace that deadline
  with a bounded ten-minute stream deadline; a healthy long answer is not aborted at
  15 seconds. Two failures open a 45-second cooldown. Explicit user stop remains
  distinct from transport timeout.
- Attachments, files, images, media, content references, unsafe routes, stale templates,
  and changed conversation context always use the official page path.

## Device evidence

Xiaomi `e0d909c3` accepted APK `1.1.1365 (1386)`, adapter `206`, through the production
friend-chat composer in an isolated blank conversation. The first cold transaction used
the official fallback while no reusable template existed and completed in 24.557 seconds.
The second transaction detected one-time dynamic proof, selected the official fallback
without attempting an invalid private POST, and completed in 22.734 seconds. Both replies
reached terminal stream state, the previous conversation and draft were restored, and no
cookies, application data, credentials, headers, or private conversation content were
cleared or emitted.

This evidence completes the transaction coordinator and its safe fallback. It does not
claim that the current dynamic-proof ChatGPT Web contract supports direct private POST.
Revisit direct dispatch only if a current official request is proven reusable or a fresh
proof transaction can be obtained without exporting credentials or splitting page state.

## Current contract audit (2026-09-05)

A production-composer send on research Release `1.1.1538`, adapter `259`, observed
successful `conversation/init`, `f/conversation/prepare`, and sentinel preparation,
followed by HTTP 200 `text/event-stream` from `f/conversation`. That write included
`openai-sentinel-chat-requirements-prepare-token`, in addition to proof and turnstile
headers. Header names and response metadata were observed, never their values.
The first send chose `official_fallback:template_unavailable`; the exact-marker
reply assertion failed, so the two-turn smoke is not counted as passed. Its finally
block restored the prior blank conversation and input.

Relay revision 16 / policy revision 12 / adapter 260 harden the existing owner:

- All nonempty `openai-sentinel-*` request headers make a template non-reusable,
  including new header names; no dynamic proof is stripped or replayed.
- A new official write immediately revokes old eligibility. Asynchronous cloning
  cannot restore a template after a newer write, invalidation, or disposal.
- Early stream receipts are retained only while that request is being captured.
  An old assistant or another conversation cannot settle the pending turn.
- A synchronous exception after entering fetch is an indeterminate write, not
  permission to send it again through the official composer.
- Late responses cannot reverse a timeout or penalize a newer transaction.

Focused executable coverage is in `test-chatgpt-web-text-transaction-lifecycle.js`;
the original transaction, send-settlement, and private-stream suites remain in use.
These changes harden dispatch eligibility and recovery, not a claimed successful
replacement for the current fresh-proof official send transaction.

## Published hardening acceptance

Release `1.1.1539 (1539)`, source `c5873cdeb`, adapter `260`, was published and
installed with data preserved. APK SHA-256:
`64b266c05d535ba871d5599a3789e72f77095a51500e1ba7e8ab89ce0851a28e`.
Research mode and Android debug mode are disabled in this artifact.

The production `social_ai` composer sent one synthetic arithmetic question and
received one matching answer in 18.654 seconds. The receipt was
`official_fallback:template_unavailable`, the provider returned to ready with
streaming false, and the original blank conversation and empty draft were restored.
This verifies the native UI and official transaction remain usable after hardening;
it is not a direct-private-send latency or performance result.

All 11 lifecycle regressions fail against source `9c38249a6` and pass after the
fix. The original text transaction, send settlement, and private stream transport
suites also pass. Android Release compilation, vital lint, publication, and
whitelisted-device replacement installation succeeded. Direct private POST with
the current fresh-proof contract remains unverified and is not marked complete.
