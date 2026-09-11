# Original Canvas History And First Share

| Capability | Code | Verification | Completed |
|---|---|---|---|
| `android_chatgpt_private_canvas_history_restore_v1` | implemented | offline tests; grouped device acceptance pending | false |
| `android_chatgpt_private_canvas_first_share_v1` | implemented | offline tests; grouped device acceptance pending | false |

This extends the [original native editor](chatgpt-original-canvas-save-20260912.md),
not another test page or shared-link manager. Reuse these modules in the next
acceptance batch; do not rediscover the protocols. No APK was assembled or
installed in this batch. No personal document, account, Cookie or proxy changed.

## Observed Protocol

Public website assets from the previously observed `web_20260911_b` build were
read as source, not executed outside the original page runtime:

- `ac476d6c-n5q4k6410vjb4971.js`, SHA-256
  `2ac259a5c38a21c79a43fa49b1b6dab3c94a4081290f362d3cf199558f6b4af8`.
  `pa` reads `/textdoc/{textdoc_id}/share`; `da` creates it with a bodyless POST
  and unwraps `shared_textdoc`. `ua(id)` is the exact share query key
  `['canvas', 'textdoc', 'share', id]`. No guessed audience field is submitted.
  `za` reads history with `before_version`, unwraps `previous_doc_states`;
  `Ra` pages to the oldest returned version, stopping at one.
- `conversation-small-ft205i7yqa6zc2nj.js`, SHA-256
  `a89952420338983e104e94be5fae8e9ae7a4169b9ea8a4ed30e639641e14f456`.
  `Ler` (export `evt`) uses `Rer` to POST `/textdoc/{textdoc_id}/restore` with
  `{version: currentVersion, restore_from_version: historicalVersion}`.
  The response supplies the new version; the website invalidates the original
  `[conversationId, 'textdocs']` query. `bVr` decodes shared-textdoc metadata.

The transport uses the existing authenticated, page-same-origin `/backend-api`
request owner. Read paths do not require composer DOM or editor runtime imports.
Writes reuse the existing observed edit-store and pending-work checks; they do
not manufacture optimistic website state or reset an edit queue.

## Implementation

- `chatgpt_web_private_canvas_documents.js` v3 adds history, restore and original
  share operations to the same canonical `canvas_document` command. History
  pages have short-lived scoped selection tickets, complete inert text and
  comments. Foreign, duplicate, unordered or malformed versions are rejected,
  not shown as an empty history. At most two history selections are retained.
- Restore rechecks both the current original and the selected historical page,
  then submits exactly one version-conditional request. Readback must match the
  selected historical title, type, complete content, comments and new version.
  Existing UTF-16/codepoint conversion is reused. A timeout remains an unknown
  write; GET verification or explicit comparison never replays the POST.
- `chatgpt_web_private_canvas_document_sharing.js` reuses the shared-content
  decoder. Only an explicit `shared_textdoc: null` means no share. Missing or
  malformed metadata stays unknown. Existing public links are returned at their
  published version without silently publishing newer content. Restricted links
  cannot become copyable public links or have their audience changed implicitly.
- First publication checks the original version and clean website queue, then
  confirms both POST response and fresh share GET against the selected original.
  Unknown publication can be read-verified; explicit acknowledgement is read-only
  and cannot clear an unrelated unknown content write. No automatic resubmission.
- `ChatGptWebCanvasManagementProtocol` adds strict typed history/share display
  payloads. Complete bodies use the display channel, not diagnostic receipts.
  Diagnostics remain counts and status only. Existing-link update/revoke are
  left in the existing shared-link manager.
- `WebChatCanvasManagementCoordinator` opens from history/share icon buttons in
  the production editor. History supports older/recent pages, full inert preview,
  and a separate restore confirmation. Share supports lookup, copy, explicit
  first-publication and unknown-result verification. Dirty or stale drafts block
  restore/publication. Closing the editor cancels the child generation and poll;
  late responses cannot reopen a closed editor. No WebView navigation or reload.

## Offline Evidence

Seven Node suites passed **186 tests**, zero failure/skip/cancellation: original
policy/documents/actions, shared content/publish, history and first share.
The new history/sharing cases cover paging, Unicode comments, identity/version
drift, dirty website state, strict confirmation, response/readback mismatches,
lost responses and read-only recovery. Native source contracts check production
icon wiring, confirmation IDs, inert presentation and close-generation cleanup.
All fixtures are synthetic.

Android Release compilation and `:app:testReleaseUnitTest` passed with **34 JUnit
tests**, zero failures/errors/skips: CanvasManagement 6, CanvasDocuments 5,
CanvasContent 6, OperationReadiness 10 and CanvasDraft 7. The grouped command
`canvas-management-release-tests-20260912-042328-211` completed in 258.8 seconds;
Gradle reported `BUILD SUCCESSFUL`. Source-size guard passed 16 files;
documentation guard passed three files with the existing near-limit warning on
the remaining-work map (which shrank). These are not device/visual acceptance.

## Next Acceptance

Use one grouped APK and a disposable owned original Canvas. Open it from the
production conversation menu; test history paging/preview, cancel restore, then
confirm one restore and verify its new version/content/comments. Test cancel
first-share, confirm one publication, copy the resulting link, and retain the
original. Restore the starting document state and view. Restricted/workspace
sharing, comment authoring and rich formatting are not implied by this batch.
Google and thermal work remain later phases; the broader Goal stays active.
