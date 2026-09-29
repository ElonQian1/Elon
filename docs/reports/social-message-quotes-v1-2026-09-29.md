---
version_status: current
reviewed_at: 2026-09-29
implementation_status: verified
---

# Compact Social Quotes Acceptance

Requirement: [Social Message Quotes V1](../requirements/social-message-quotes-v1.md).

## Implemented

- Win and Android keep typed replies separate from removable source previews.
- Sent replies show the author, bounded summary and optional cover below the bubble.
- Source navigation uses the loaded message when available and a snapshot otherwise.
- Historical Markdown quotes remain readable; AI Markdown blockquotes are not reclassified.
- Quote snapshots are server-owned, same-scope and revision-checked, and stored atomically.
- Withdrawn/deleted sources do not expose the stored body or attachments to readers.
- Normal AI history and selected-message analysis retain quoted context without adding
  unrelated group messages to a selected-only request.
- PWA renders received structured quotes using the shared theme.

## Evidence

- Production SQL harness: 31 tests pass, including quote isolation, revision, recall,
  rollback, AI context and selected-only context. Command receipt: `quote-selection-context`.
- Android codec and native layout: 5 tests pass, including narrow layouts, enlarged fonts,
  recycled rows and the 48dp cancel action. Receipts: `quote-android-layout`,
  `quote-android-final` (after composer inset adjustment).
- Win production TypeScript/Vite build and bundle gates pass. Receipt: `quote-pc-build`.
- Browser quote interactions pass at 1280/390/320 widths with dark/light semantic tokens.
  Tests cover cancellation, retained drafts after rejection, retry, body/source separation,
  legacy replies, source navigation and horizontal overflow. Receipt: `quote-ui-final`.
- Existing social operations, chat recovery and full chat parity checks pass after updating
  the former Markdown-serialization expectation. Receipts: `quote-chat-recovery`,
  `quote-chat-parity-final`.
- Xiaomi Release 1.1.1834: semantic `quote_draft` and `quote_cancel` pass in the authorized
  group. The draft remained intact, cancellation only removed the quote, then the synthetic
  draft was cleared. No group message was sent. One native frame was inspected locally;
  its tighter corner spacing prompted the final composer inset adjustment.

## Delivery And Limits

- Main feature: `9618edfd4`; selected-context compatibility: `7f745f795`.
- Win frontend and backend 0.3.1796 deployed successfully.
- APK 1.1.1834 published and installed without clearing data on the registered Xiaomi;
  the registered Honor was offline. APK SHA-256:
  `e5427a24147a8a79bff92731a5b48288294318695c2f25cd30ced99bfd637f54`.
- Final inset APK and selected-context backend release receipts are pending below.
- The full server unit-test target has unrelated pre-existing E0364 visibility errors
  in compute-plugin test fixtures. This is not reported as a full-suite pass; the production
  Release build and focused production-source SQL harness are the verified alternatives.
- Browser light colors are semantic-token compatibility coverage, not a newly added PC
  theme switch. Native final-release recheck and sent-message UI are separate evidence:
  no production chat was modified merely to manufacture an acceptance screenshot.
