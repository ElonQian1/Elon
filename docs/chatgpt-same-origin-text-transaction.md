---
capability_id: android_chatgpt_same_origin_text_transaction_v1
implementation_status: completed
verification_status: device_verified_v1_1_1365_adapter_206
production_default: true
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
- Internal requests time out after 15 seconds. Two failures open a 45-second cooldown.
  Explicit user stop remains distinct from transport timeout.
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
