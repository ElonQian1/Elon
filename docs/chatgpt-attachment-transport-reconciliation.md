---
capability_id: android_chatgpt_attachment_transport_reconciliation_v1
implementation_status: completed
production_default: true
official_page_authoritative: true
---

# ChatGPT attachment transport reconciliation

The native friend-chat attachment flow continues to give the selected file to the
persistent official ChatGPT WebView. The official page owns its file input, upload
protocol, React state, prompt dispatch, and final conversation state. Android does not
copy cookies into a separate HTTP client and does not replay an upload or send request.

Adapter `207` adds a document-start observer that remains idle until the native file
picker is requested. While armed, it recognizes only bounded same-origin POST lifecycle
stages used by the official attachment flow. Events contain only a protocol version,
monotonic sequence, state, and completed attachment count. They never contain a URL,
opaque identifier, file name, file bytes, request or response body, headers, cookies, or
credentials.

The existing DOM attachment chip remains valid completion evidence. When the official
page does not render that chip, a successful transport completion can also release the
already-reserved single send slot. Dispatch still requires the current official snapshot
to be composer-ready and not streaming. Duplicate, stale, malformed, unsupported, or
failed observations cannot dispatch. The existing 120-second timeout and official DOM
reconciliation remain the fallback.

## Verification

- Node contract covers early installation, late bridge binding, Fetch and XHR, wrong
  origin/method rejection, duplicate file suppression, multi-file monotonic counts,
  cancellation, and payload privacy.
- Kotlin tests cover protocol bounds, stale sequence rejection, multi-file gating,
  stable-snapshot gating, failure fallback, and single-owner reserved dispatch.
- Xiaomi device acceptance on Research APK `v1.1.1373 (1394)`, adapter `207`, used only
  the repository fixed text fixture. The official upload completed, native attachment
  phase became `completed`, pending count returned to zero, exactly one user turn and
  one assistant turn were observed structurally, the fixture was removed, and the
  original conversation was restored. No private conversation content was emitted and
  no cookies or application data were cleared.

This capability is complete and production-default. Future work should reuse it rather
than adding another attachment uploader unless current regression evidence proves the
official upload owner is no longer viable.
