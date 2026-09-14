# Temporary Writing Block Save

Status: implemented / offline verified; published in 1723, device acceptance pending. This extends
the existing `android_chatgpt_writing_block_save_v1` scope, not its completed
ordinary-conversation acceptance. The later [grouped APK delivery](reports/chatgpt-writing-grouped-acceptance-20260914.md)
does not count as temporary-conversation UI acceptance.

## Scope And Evidence

The retained `web_20260912` source `a965fc59-fzrm5l4zirdbhwph.js`, SHA-256
`752c85e9623229704c208167584c5b7a6e8f18410e6258713d2de7d483a62e19`, supplies
`rc -> nc`: the writing save uses the existing server conversation ID, source
message, block index/ID and content. Its non-library branch does not dispatch a
privacy/history mutation or use a separate temporary endpoint. The existing
public-source AST/hash test now checks that boundary explicitly. Downloaded
website code is parsed, never executed.

This proves the reviewed client contract, not live server support for every
temporary chat. A fresh same-origin GET must still return the exact conversation,
selected branch and block with `is_do_not_remember=true`; an explicit
`is_temporary_chat=false`, project/shared owner or Library-backed block rejects
admission. A missing/unavailable history response is not permission to write.

## Production Path

The existing native block editor and Save to Website command are reused.
Native snapshots omit URL queries, so the writing-specific path parser can turn
the ChatGPT homepage into a **temporary candidate**. General conversation
navigation is unchanged. That candidate is not an ownership proof: the page
must be at exactly `/?temporary-chat=true`, have an authenticated personal
workspace and exactly one officially selected temporary conversation with a
valid server ID, confirmed privacy state and completed initialization.

The existing temporary control's owner proof is reused, without searching the
DOM again. If it has not been established, preparation remains unavailable;
this is not a claim of composer-free cold startup. Message rendering only reads
already-loaded modules and does not initiate a fetch or import.

The ticket captures that conversation object, server ID, account, document,
route and leaf. A second temporary chat at the same homepage cannot use it.
Normal draft, streaming, dictation and competing-write protections remain.
Before dispatch the current server/local block must still match. One confirmed
POST is followed by readback and the existing single-node metadata update;
unknown outcomes remain pending and can only be verified, not resent.

The latest completed stream can otherwise overwrite newly saved content during
native snapshot merging. Its new narrow reconciliation entry replaces only
the same message in the same completed temporary stream, after the writing
owner confirms the server result. Active streams and mismatched conversations
are never replaced. An older edited message does not replace the latest stream.
Failure stays `writing_saved_sync_pending`; it is not reported as native success.

After success, only the current native snapshot is emitted. There is no normal
history prefetch, directory insertion, Library write, privacy toggle, page
reload or extra chat send. The existing temporary-cache policy stays unchanged.
Local editing/export remains independently available when cloud preparation fails.

## Verification And Remaining Work

- `writing-temporary-targeted-20260914-131046-213`: 128 Node tests passed,
  zero failures/skips, 30.9 seconds. Includes current public-source contracts,
  ordinary/project/widget/Library save regressions, temporary owner/branch/
  privacy/Library rejection, unknown-write readback and real stream-module
  composition. Synthetic data only; no account request or microphone use.
- `writing-temporary-owner-regression-20260914-132136-864`: 58 existing temporary
  owner/selection and fresh-send tests passed, zero failures/skips. Total for
  this batch: 186 Node tests. Source size and document guards also passed.
- Protocol, policy and context versions are 1 (wire unchanged), 4 and 7;
  writer 4, text-block parser 7, stream observer 24, APK adapter 391.
- Kotlin route/receipt tests passed in the grouped Release compilation and
  51-test run `writing-grouped-native-tests-20260914-132807-441` (315.2 seconds,
  zero failures/errors/skips), including six writing-protocol tests. This is
  not a production UI pass and no APK was built or installed for that run.
- Initial phone discovery returned no device; the later grouped run found an
  online handset with an unsent draft, which was left untouched. Live temporary history accessibility,
  native edit/save/reopen and history/Library non-insertion still need one
  controlled acceptance case. Do not reuse ordinary save 1692 as proof.
- Shared-copy writing remains outside this scope. The completed local editor,
  ordinary save and Word export are reused; PDF device acceptance remains
  separate in [the PDF report](chatgpt-writing-blocks-pdf-export.md).
