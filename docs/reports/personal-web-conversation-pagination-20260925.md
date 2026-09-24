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

## Released Artifacts And Live Acceptance

Source `19fe4eb900008f5e3e617ffb1e99f1e31f8a7a1c` passed the formal NodeAgent
and AndroidFeature completion checks. Win `0.3.69+19fe4eb` was activated and
its exact identity read back from the running node. APK `1.1.1811` was published
and remotely hash-verified: SHA-256
`f7c641177229012e96de79f908b2b8e736ace66df07b7be57a5c3fdc308b2036`.
Both registered debug phones were offline; this release was not installed on them.

The actual persisted Codex MCP configuration discovered its three tools and
read the authorized conversation to the last output cursor in 69,135 ms:

- 17 pages, 34 blocks, 25,347 Unicode code points, and 21 message IDs with blocks.
- The projection's candidate message counter was 90; this is not proof of 90
  complete visible messages. Some candidate messages had no supported content.
- Eight attachment metadata records; attachment bytes were not read.
- `text_complete=false`, `multimodal_complete=false`;
  gaps were `unsupported_message_content` and `attachment_bytes_not_read`.
- Ordered block SHA-256:
  `ca969a9e980f2f66ff2736110d4d7245ce7653d4cb5a53f930418ab6866275b7`.

With zero active client tasks, the installed desktop process was stopped for
startup acceptance while the background node remained online. An immediate
attempt encountered the old host lease and returned `win_action_timeout`.
After the host list became empty, the same configured MCP automatically launched
the installed desktop and ChatGPT WebView, reused login, and read the same 17
pages in 74,637 ms with the identical block hash. This proves desktop/WebView
startup from an absent host; it does not prove full node-off cold startup.

The first parallel Win build was terminated by the 600-second silent-output
guard under memory pressure. A premature retry correctly rejected the APK
publisher's temporary version edit. After APK completion restored the tree,
the sequential Win build passed with a bounded 1,800-second silent-output limit.

## Remaining Gaps

The authenticated transport and original `invalid_branch` failure are resolved.
Full-text completeness remains unverified because unsupported candidate content
is still reported. Its exact response shapes were not exported or diagnosed;
the public schema's helper/reasoning types alone do not prove which live records
caused the gap. Do not discard this flag or label those records harmless without
bounded structural evidence. Attachment bytes, APK device reading, full node-off
cold startup and actual Claude model invocation remain unverified. No Claude CLI
was available on this PC's PATH. The separate full UI adapter bootstrap was not
repaired by this reader change. The feature remains `implemented`, not `verified`.
