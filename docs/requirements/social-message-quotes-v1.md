---
version_status: current
reviewed_at: 2026-09-29
implementation_status: implemented
---

# Social Message Quotes V1

## Goal

Match the requested WeChat-style reply hierarchy in Win and Android social chats.
Keep the user's reply independent from a compact, removable source preview.
After sending, show the quote outside and below the reply bubble, with author,
up to two lines of summary, a muted vertical rule and an optional small cover.

## Scope And Contract

- Group and friend chat composers use structured `quote_source` identity and revision.
- The server resolves authorized source messages and stores a single-level snapshot
  in the same transaction as the reply. Clients cannot supply another user's text.
- Keep ordinary message text, attachments, AI reply paths and work-mode composer behavior.
- Old Markdown-prefixed social replies remain readable without nesting new previews.
- Recalled or removed sources become unavailable; cross-group and stale references fail.
- AI history includes the referenced context instead of silently losing it with the UI change.
- PWA can read the same structured quote metadata.
- No global theme rewrite, account/login change or private-provider transport redesign.

## Acceptance

1. Choosing/canceling a quote does not overwrite the typed draft.
2. Network failure retains the text and quote; confirmed success clears only that draft.
3. Replies display the quote below the bubble; tapping locates the source or opens its snapshot.
4. Text, images, links and record cards have bounded previews, not duplicate full link cards.
5. Small-screen and enlarged-font layouts remain readable; Android cancel target is 48dp.
6. Server tests cover source authorization, revision, recall, transaction rollback and AI context.
7. TypeScript build, browser interaction tests and Android targeted tests pass before release.

## Verification Entry Points

- `scripts/test-social-quotes-ui.cjs`: production React components with synthetic data.
- `server/tests/group-ai-selection-harness`: production quote SQL and AI selection tests.
- `android/app/src/test/kotlin/com/elon/app/socialquotes`: codec and native layout tests.
- Runtime screenshots and command receipts are temporary artifacts, not private chat fixtures.
