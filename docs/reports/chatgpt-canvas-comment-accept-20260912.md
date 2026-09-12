# Original Canvas Comment Acceptance

Status: implemented candidate, offline verified; production-phone acceptance
pending. The parent `android_chatgpt_private_canvas_original_edit_v1` remains
partial. This builds on [AI editing](chatgpt-canvas-generation-20260912.md),
not another generic sender or a DOM button automation path.

## Provider Evidence

The reviewed `web_20260912` editor calls its existing Canvas hook with the
original document version. Comment acceptance first awaits a version-bound
comment DELETE with `reason=accept`; a rejected DELETE aborts the AI step. It
then uses the comment's original text and UTF-16 range, `accept_comment`, `edit`
and selection metadata. It does not substitute `dismiss` or a raw body save.

| Public source | SHA-256 |
|---|---|
| `ac476d6c-foq8estmw3bbr7op.js` | `f341cbd1ebc80c14838f6bada429d1826a5a78577f094f5be452ca7663c3c895` |
| `conversation-small-h1dtzoris1y9588z.js` | `da08c64c132306779e09ba89cac64fa560b120e7560ffdc29b3ce5a0b8ccd67e` |

`Ha.ot` awaits `Xe(n, ACCEPT)` then calls `k(...)`; `k=kr(s,D)` comes from the
same official generator used by the preceding batch. Conversation export
`Cvt/Iir` calls `kir`, whose `safeDelete` uses
`/textdoc/{textdoc_id}/{version}/comment/{comment_id}` and returns `.version`.
The public-source test checks these AST definitions and hashes without executing
downloaded provider code in Node. Generator and Intl evidence stays in the
preceding report.

## Native Behavior

- Production original-editor comments expose **accept and rewrite**, reanchor,
  and dismiss as distinct commands. Unsaved native text/anchor changes must be
  saved before acceptance; dismissal still preserves drafts as before.
- Only the selected original comment ID crosses the native command boundary.
  Its text and range are derived from the canonical document, not a caller's
  arbitrary replacement prompt or an invented range.
- The actual official hook and permission are checked before deleting. The
  canonical original is checked before the write and after its version receipt.
  The existing hook preserves the original version, mode and hidden context.
- One DELETE is followed by at most one AI dispatch. Pending work blocks
  competing native sends/regeneration in the same conversation. No write is
  automatically replayed, including through the official page.
- If removal succeeds but the AI command is not dispatched, read-only version
  verification offers **continue rewriting**, **stop**, or **later**. Continue
  sends only the unsent AI command after rechecking identity, source and version;
  it never removes the comment twice. Stop explicitly acknowledges the result
  and leaves the current body intact.
- An AI command already invoked with an unknown result cannot use that resume
  or stop path. It remains pending until its exact original-comment turn and
  terminal reply are observed. The comment-removal version alone is not proof
  of a rewritten body; no-change completion remains a separate outcome.
- The original body stays visible. Existing bounded generation verification is
  reused, and a partial-result decision stops automatic polling while waiting
  for the user. Closing the editor disposes its native coordinators.

Stable IDs: `web-chat-canvas-comment-accept`, `-accept-confirm`, `-accept-cancel`,
`-accept-resume`, `-accept-stop`, `-accept-later`, and `-reanchor` share the
`web-chat-canvas-comment` prefix. Existing dismiss IDs are preserved.

## Verification And Delivery

- `canvas-comment-accept-final-contracts-20260912-104237-247`: **552 Node tests
  passed**, zero failures/skips. Includes Canvas save/history/export/share,
  generation, canonical actions, native text and regeneration contracts.
- Acceptance-specific cases cover strict commands, original UTF-16 selection,
  long original suggestions, permission/version changes, rejected DELETE,
  malformed receipt, response loss, duplicate taps, partial-success resume,
  explicit stop, uncertain AI dispatch and identity changes.
- `canvas-comment-accept-release-check-20260912-104040-276`: Android Release
  compilation and **26 JUnit tests passed** (comment acceptance 3, generation 3,
  draft 13, original document protocol 7), no failures/errors/skips. The logged
  Gradle check completed in 321.5 seconds without a timeout or stall.
- No APK assembled, published or installed in this batch. Phone proof is still
  required for the live two-step provider call, result-version adoption, partial
  recovery dialog, and comment controls on narrow/landscape screens.

## Next Work

Faithful Markdown/code export remains a code gap with exact source leads in the
[export report](chatgpt-canvas-original-export-20260912.md). Then perform the
grouped original-editor acceptance round, preserving the proven native audio,
subtitles, dictation and read-aloud. Keep Google after the ChatGPT gate in the
[remaining map](../web-ai-private-native-remaining-batch.md).
