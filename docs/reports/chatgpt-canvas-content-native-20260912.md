# Native Shared Canvas Content

## Scope And Status

`android_chatgpt_private_canvas_shared_content_v1` is a read-only source viewer
in the existing production Canvas shared-link manager. It does not duplicate
the manager, publish or edit a Canvas, restore a version, execute code, or
delete the original document. The previously accepted list/Copy/Keep-link case
remains [unchanged](chatgpt-canvas-shared-links-20260911.md).

The subsequent [existing-link update batch](chatgpt-canvas-edit-publish-contract-20260912.md)
reuses this viewer and records the now-resolved original editor/save owner. That
separate source capability does not broaden this read-only acceptance scope.

- `transport=page_private_http`; identity stays in the existing WebView.
- `code_status=implemented`; initially adapter 358, accepted with adapter 359.
- `verification_status=device_verified` for native source read and return.
- `android_chatgpt_private_canvas_shared_content_v1:read_and_return`:
  `completed=true`, available through the normal production entry on APK 1677.
- [1677 acceptance](chatgpt-canvas-production-1675-20260912.md#follow-up-landscape-and-shared-content-on-1677)
  matched native/canonical body length and restored the same conversation/draft.
  Source Copy and write operations were not exercised; full Canvas completion
  is not claimed. The initial offline evidence below remains historical.

## Website Evidence

The currently observed public conversation asset
`conversation-small-ft205i7yqa6zc2nj.js`, SHA-256
`a89952420338983e104e94be5fae8e9ae7a4169b9ea8a4ed30e639641e14f456`,
uses `safeGet('/textdoc/shared/{shared_textdoc_id}')` and unwraps
`shared_textdoc`. Its `bVr` mapper supplies shared ID, `name`, `content`,
`textdoc_type`, nullable `version`, `access`, moderation and API-key-detection
flags. The existing provider request client supplies the `/backend-api` prefix.
The native reader reuses that observed GET, not a guessed source-document URL.

The live shared page imported `canvas.shared._sharedTextdocId-cjaujaam.js`
(SHA-256 `e9a10d97faade4ae706eb2662f323b5ff549e829ecbec4e1d5de398c8710e69e`)
and renderer `d423cc1e-e6t7dw9ixo0vzzqk.js`
(SHA-256 `7efe7460ebacf687ed4cf28e6f96c10e789295f28ae42f2e5b0e6084e3b6da71`).
The renderer distinguishes document/code and can execute code with separate
permissions. This native viewer deliberately shows inert source text only;
it does not reproduce the website's execution or rich preview permissions.
Public assets were downloaded without credentials and parsed, not executed.

Further evidence exists for conversation textdoc lists, diff reads,
version-bound restore, comment deletion and PDF/DOCX export. These are separate
operations. `conversation/textdocs/infer_metadata` infers a temporary document's
metadata; it is not a persist/create endpoint. The initial read-only batch had
not traced the edit/save owner; the linked subsequent batch records that owner
separately. No write protocol was invented for this viewer.

## State And Presentation

1. The existing account list issues the account/document-bound selection ticket.
   `read_account` accepts Canvas only and an exact personal row in that list.
   It neither needs composer readiness nor mutation confirmation.
2. The existing bounded same-origin JSON reader performs GET with a 7-second,
   1 MiB response limit. At most two documents are cached for 60 seconds;
   account/document drift, auth rejection and share invalidation discard them.
   No background polling or second WebView is added.
3. Only public, non-blocked, correctly typed content is admitted. Source text
   over 128 Ki UTF-16 units is rejected explicitly, never truncated. Unknown
   optional versions stay null. Workspace/private links and detected restrictions
   are not silently treated as ordinary public documents.
4. Canonical `canvas_shared_content` display events carry the full text to the
   native model. The command ledger contains only `share_canvas_ready` or a
   bounded error code. MCP `canvas_content` contains request ID, length, type and
   version only, never a title, body, URL, cookie or request headers.
5. Native state accepts only the latest pending share request and discards data
   after history/document changes. The view also checks request ID and selected
   link ID. Back during loading invalidates the callback epoch, so a late result
   cannot reopen the viewer over the user's restored link dialog.
6. The existing link dialog gains `web-chat-canvas-share-view`. The read-only
   scroll surface has selectable title/source text, Copy source and Back to
   link controls. HTML/code remains literal text; no renderer, script, remote
   resource or implicit URL activation runs. Existing link Copy/Revoke/Back to
   list remain available and do not publish or modify content.

## Verification

Four focused Node suites pass **166 tests**. Cases cover the exact GET and
schema, receipt/display separation, hot cache/expiry, selection and identity
drift, malformed/restricted/oversize content, optional version, failed-read
retry, missing display channel, legacy writer upgrade and existing sharing.
Both production share UI source contracts pass, including inert rendering,
selection/request correlation and cancellation of late UI updates.

Release production/test compilation passed, with **21/21** tests across four
Kotlin suites (six new content cases). The final Gradle log ends in
`BUILD SUCCESSFUL`; all four JUnit XML files have zero failures/errors/skips.
Evidence: `canvas-content-release-tests-final-20260912-020503-906` and
`canvas-content-node-final-20260912-020534-459`. The initial check used a
nonexistent flavor task; a subsequent mixed source snapshot also failed test
compilation. Final verification froze sources and used `testReleaseUnitTest`.
Do not edit production sources during a compile and then trust tests against
an earlier class snapshot. No APK is packaged per the user's grouped-build
workflow. An installed-1673 read-only list command timed out during research;
that does not verify or disprove the new, uninstalled content reader. No
content, link, account token or header was exported; no link was revoked.

Native View/full body length/Back and conversation/draft restoration passed on
1677. Do not repeat that accepted path absent a regression. Source Copy, hot
reopen and cancel during read remain offline-tested or unexercised, not part of
that device result. Actual edit/publish/revoke and rich preview remain separate
unfinished scopes.
