---
version_status: current
reviewed_at: 2026-09-25
---

# Authorized Conversation Pagination Repair

[Reader contract](../personal-web-conversation-reader.md) · [Previous delivery](personal-web-conversation-codex-win-20260924.md)

## Live Baseline

After the user completed official WebView login on Win `0.3.69+6f06ab0`,
the persisted Codex stdio MCP discovered all three reader tools. Reading the
authorized conversation reached projection and returned `invalid_branch`.
This is evidence that login was no longer the immediate blocker, not evidence
of a successful full conversation read. No private content or credentials are
retained in this report or the regression fixtures.

## Protocol And Repair

The official public asset `https://chatgpt.com/cdn/assets/4813494d-cfz7xrlmg5sd5itt.js`
was retrieved on September 25 and matched the repository's reviewed September 22
SHA-256 `2d3c09d0f0ff619edb0323650d4189bce09d8fd31c75ccbc7ac7fc3044fdad8c`.
Its `E0t` reads `/conversations/{conversation_id}` and treats `messages` as
chronological branch history. `D0t` reads `/conversations/{conversation_id}/messages`
with `before` and `include_has_versions`; `C0t` uses
`page_info.has_previous_page` and `start_cursor`. `_0t` constructs a synthetic
root and consecutive parent links rather than following message metadata parents.
The resource was inspected as text, never imported or executed.

Projection v2 recognizes that explicit pagination contract, preserving the
server's chronological selected branch. Legacy full mappings still require a
valid, unambiguous parent chain. Reader v3 follows older-page cursors, prepends
messages, rejects repeats, overlapping message IDs, mismatched conversation IDs,
malformed completion flags and account changes. Collection has time, page, size
and message bounds. A repeated first-page read detects branch or content changes
during multi-page collection. No snapshot is emitted until the source signals
completion and the current leaf matches the last message.

The existing account-bound output cursors continue paging the immutable in-memory
snapshot. Attachment bytes remain an explicit gap. This does not repair the
separate full UI adapter bootstrap or enable arbitrary evaluation.

## Verification

All 22 focused Node tests passed, including five new pagination cases covering
omitted hidden parents, older/empty pages, opaque cursor escaping, full output
ordering, unstable source rejection, account drift and bounded collection.
Existing authorization, Unicode, attachments, stdio/HTTP, persistent registration
and Win startup tests passed unchanged.

At source preparation, live acceptance of the repaired Win/APK builds is pending.
Real cold startup, Claude model invocation and attachment-content reading remain
separate acceptance items; passing substituted transport tests does not establish
those outcomes.
