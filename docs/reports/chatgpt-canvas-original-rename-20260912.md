# Original Canvas Rename

Date: 2026-09-12. Parent capability:
`android_chatgpt_private_canvas_original_edit_v1`.
Scope: rename an owned original Canvas from the existing production native editor.
Code is implemented and offline verified; device acceptance and APK publication
are pending. This does not mark the complete Canvas capability accepted.

## Reviewed Contract

The public editor asset `ac476d6c-foq8estmw3bbr7op.js`, SHA-256
`f341cbd1ebc80c14838f6bada429d1826a5a78577f094f5be452ca7663c3c895`,
contains the `zi({textdocId,newTitle})` owner. It calls
`safePost('/textdoc/{textdoc_id}/rename')` with `{title}` as its request body.
The existing client supplies `/backend-api`. This is an original textdoc ID,
not a shared-link ID. No content, comment list or version is posted.

The caller consumes no response body and invalidates the conversation's
`[conversationId,'textdocs']` query. Its associated mutation uses the same key
and variables `{textdocId,newTitle}`. These facts are checked against the exact
public asset; no private session data or credentials were captured. Public
source evidence alone is not a successful account/device request.

## Implementation

- Reuse the versioned Canvas documents transport, strict selection tickets,
  session binding, bounded requests and write-uncertainty owner. One confirmed
  rename POST is followed by a canonical textdocs GET. A 2xx/204 without matching
  readback is not success. Unlike the official caller's retry configuration,
  this transport never retries a dispatched write automatically.
- Readback must match the requested title, original identity/type/body/comments
  and a nondecreasing version. Renaming need not create a new content version.
  Lost responses are resolved through the existing GET-only verification flow;
  a mismatched result requires explicit acknowledgement, not a second POST.
- The existing website-edit observer also rejects a pending official rename for
  this exact conversation and document. Body-save guards remain in place. Other
  documents' mutations do not block this document.
- The production native editor gains a rename icon and an immediate local input
  dialog. Cancel/no-change sends no write; confirmation revalidates before POST.
  Stable controls are `web-chat-canvas-editor-rename`,
  `web-chat-canvas-rename-title`, `web-chat-canvas-rename-confirm` and
  `web-chat-canvas-rename-cancel`. No test page or WebView navigation is added.
- A confirmed title-only change updates the draft base without replacing unsaved
  native body text, comment positions or anchor-repair requirements. Version,
  identity or source drift cannot silently overwrite the user's draft.

## Verification

Ten focused Node suites passed **258 tests**, zero failures, skips or
cancellations. Run: `canvas-rename-combined-contract-20260912-083652-657`.
Coverage includes the exact public source witness, canonical dispatch, source
and account drift, strict confirmation, malformed names/Unicode, pending web
rename, single flight, body/comment preservation, unknown writes and GET-only
recovery. Existing Canvas read/save/history/sharing contracts are retained.
Fixtures are synthetic. Native UI wiring checks are source contracts, not
rendered-device acceptance.

Android Release compilation and five targeted JUnit suites passed **38 tests**,
zero failures/errors/skips: CanvasContent 6, CanvasDocuments 6, CanvasManagement
6, OperationReadiness 10 and CanvasDraft 10. The four new cases cover canonical
rename confirmation/schema and native draft preservation/conflicts. Run
`canvas-rename-release-tests-20260912-083623-265` took 320.9 seconds, ended in
`BUILD SUCCESSFUL`, and its XML counts were checked separately from the wrapper
exit code. Three existing Canvas smoke/UI contract scripts also passed.
Source-size guard passed 16 files; documentation guard passed two with the
existing remaining-map size warning. No APK was assembled, published or installed
for this change; grouped device acceptance remains pending.

## Remaining Acceptance

Use one grouped candidate on a disposable, owned original Canvas through the
production UI. Verify rename cancellation, one confirmed rename and canonical
title readback; keep an unsaved body edit and confirm it survives. Restore the
synthetic title and discard the unsaved test edit, then restore the starting
conversation and awake state. Original save/history restore/first publication
remain part of that same pending Canvas acceptance batch.

An unblurred title draft inside an already-open official editor is not a
documented export from the runtime dirty hook. The pending-rename guard does not
claim to introspect every website title draft. Keep this residual explicit.
Google and thermal work remain later phases; the full Goal stays active.
