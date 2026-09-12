# Original Canvas PDF and Word export

Date: 2026-09-12. Evidence report, not a capability-completion declaration.
Parent: `android_chatgpt_private_canvas_original_edit_v1`.

## Scope and status

Implemented the production native editor's PDF/Word export entry, typed original
document selection, official private export request, and the existing native
download progress/cancel/save path. No system print-to-PDF, page screenshots,
second download service, or standalone test page is used.

Offline verification is recorded below. A grouped APK and real production-phone
export remain pending. Markdown/code export and AI-generated Canvas rewriting
are not implemented by this change. Do not mark the parent capability completed.
Reuse the earlier [rename](chatgpt-canvas-original-rename-20260912.md) and
[comment dismissal](chatgpt-canvas-comment-dismiss-20260912.md) implementation.

## Reviewed source

Public asset `bc86e6a9-lyyfjzfq9uy5wnj2.js`, SHA-256
`9417a623b117d5c76343b91f9a2c6c3a0402c24d50cec8fd8a671e2fc1493eeb`:
`Xe` (export `n`, initializer `Ze`) posts `/export_doc/canvas` with
`{textdoc_id, export_type}` and `skipJsonTransform: true`. PDF and DOCX consume a
binary response; Markdown and code use separate local serialization paths.

`ne` imports `b4` (`K`) from `4813494d-gf2h57w5fiay19bd.js`, SHA-256
`6c015001732054f4143ef1922609407c540967762109dcd128bbf56706889c3e`.
`K` uses `qhe`; `Uhe` checks the Authorization header, selecting authenticated
`Br = https://chatgpt.com/backend-api` instead of anonymous `Vr`.
The implemented export requires the existing authenticated private identity.
No Cookie or private request credentials are exported to Android.

## Ownership and result contract

- A prepared export is an opaque short-lived download handle tied to the selected
  document/version, current page/document token, and account. It contains no URL
  or request headers. Preparation does not generate or save a file.
- The confirmed native selection starts one export POST through the existing
  file-download owner. Reuse its byte acknowledgements, bounded chunks, storage,
  cancellation, progress, and terminal save receipt.
- The endpoint has no version argument. Read the original before and after
  server generation; reject changes to body, title, type, version, or comments.
  Hold the existing Canvas command owner during this interval and check official
  pending edits. Do not claim this guarantees a server-side version lock.
- Reject non-200 responses, redirects, mismatched media types, invalid PDF/ZIP
  signatures, incomplete lengths, empty files, and exports over 64 MiB. DOCX ZIP
  signatures are not a full OpenXML validation or proof of successful rendering.
- Keep native drafts unchanged. When a draft is unsaved, return to editing rather
  than silently exporting the old version or automatically saving the draft.
- Consume each export handle once. A lost response or save acknowledgement does
  not retry the POST or the commit, and never falls back to a DOM click. A fresh
  user-requested export can obtain a new handle.

Production IDs: `web-chat-canvas-editor-export`,
`web-chat-canvas-export-formats`, and the reused `web-chat-file-download-*` IDs.

## Verification

- `canvas-export-final-contracts-20260912-093901-604`: 241 Node tests passed, no failures
  or skips. Covers export plus existing Canvas documents, policy, rename,
  dismissal, canonical actions, library download, and ordinary file download.
- The initial contract run exposed an incorrect fixture assumption: changing
  server type does not overwrite a still-fresh local index. Refreshing that test
  index fixes the premise; download still independently checks the original.
- A targeted failing regression reproduced a response left open when the export
  timed out during the post-generation version check. The shared byte owner now
  closes a prepared response arriving after cancellation/timeout; the final
  combined run includes this passing case.
- `canvas-export-final-release-check-20260912-093932-058`: `BUILD SUCCESSFUL`;
  42 JUnit tests passed with no failures, errors or skips: original documents 7,
  export 3, download metadata 8, download policy 5, download session 6, draft 13.
  This also compiles the final editor version-label correction.
- No new APK was assembled, published, or installed in this batch. Real output
  opening, visual layout, media type headers and production source behavior
  remain for the grouped device round.

## Remaining local-format source lead

The same reviewed export asset's unexported `G` preserves ordinary Markdown but
processes Canvas directives through `unified` (conversation exports `IAn/LAn`),
`CANVAS_REMARK_PLUGINS` (`e5d54aa7-o5mtxxnk4j8zox9y.js`, exports `n/r`),
`stripDirectivePlugin` (`1c4de3ec-ix5n1yyu8whxeib8.js`, exports `t/r`) and
`hiveLogDirectivePlugin` (`6afb0137-dqge0sx8jli56ai8.js`, exports `i/r`).
Its `Z` table owns code-language extensions and MIME types. Plain raw text is
not evidence of faithful Markdown export when those directives exist. Follow
these exact source leads rather than rediscovering or inventing serialization.

## Next acceptance

Use a disposable original document and the production editor to export PDF and
DOCX. Open both saved files, compare content, verify progress/cancel, and retain
the native draft and original conversation. Combine this with original save,
history restore, first share, rename, and comment dismissal acceptance. Preserve
the existing native audio/subtitles/dictation/read-aloud and defer Google until
the ChatGPT gate in the [remaining list](../web-ai-private-native-remaining-batch.md).
