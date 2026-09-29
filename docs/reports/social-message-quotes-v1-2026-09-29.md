---
version_status: current
reviewed_at: 2026-09-29
implementation_status: released
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
- Xiaomi Release 1.1.1835: the installed version was read back and `open_fixture` passed.
  The next `quote_draft` stopped at `foreground_package_mismatch` after the phone switched
  to another application. No further UI actions were attempted. The earlier 1834 quote/cancel
  result is not presented as a completed 1835 interaction recheck.

## Delivery And Limits

- Main feature: `9618edfd4`; selected-context compatibility: `7f745f795`.
- Win frontend and initial backend 0.3.1796 deployed successfully.
- APK 1.1.1834 published and installed without clearing data on the registered Xiaomi;
  the registered Honor was offline. APK SHA-256:
  `e5427a24147a8a79bff92731a5b48288294318695c2f25cd30ced99bfd637f54`.
- Final APK 1.1.1835 published from `9f10c2897` and installed on the Xiaomi with data
  preserved; Honor remained offline and the installation receipt contains no failures.
  APK SHA-256: `7c0eef6f1e47399be99f8d83f1b47dee5bb2bff6f85be9548f9b30e6ba944992`.
  Command receipt: `quote-apk-final-20260929-150831-762`.
- Backend 0.3.1798 deployed from `a68440b0f310bf97c12d0d9bf42726982b5f59b4`, retaining
  concurrent upstream PWA changes and the already-published Win bundle. The selected-message
  context fix is included. Health and the live Git SHA passed `check-task-complete -Kind Server`.
  Command receipt: `quote-server-final-retry-20260929-152813-425`.
- The first final-backend attempt was terminated by the wrapper's 900-second output-idle
  limit while rustc was still active. Retrying the official publisher with
  `-StallTimeoutSeconds 3600` passed; no compilation or release checks were bypassed.
- The full server unit-test target has unrelated pre-existing E0364 visibility errors
  in compute-plugin test fixtures. This is not reported as a full-suite pass; the production
  Release build and focused production-source SQL harness are the verified alternatives.
- Browser light colors are semantic-token compatibility coverage, not a newly added PC
  theme switch. Native final-release recheck and sent-message UI are separate evidence:
  no production chat was modified merely to manufacture an acceptance screenshot.
