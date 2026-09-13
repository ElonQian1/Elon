---
capability_id: android_chatgpt_fresh_attachment_text_dispatch_v1
implementation_status: implemented
verification_status: offline_verified
production_default: false
scope: owned_ready_local_and_ordinary_library_files
---

# Fresh Attachment Dispatch

September 13 extension of the [independent text sender](chatgpt-fresh-text-dispatch.md).
It reuses accepted private uploads, the existing immutable attachment submission
lease, fresh proof preparation, one-write ledger, stream projection and history
recovery. It does not add an uploader or invoke the official submit callback.
The accepted ordinary plain-text default and runtime attachment sender remain.

## Reviewed Protocol

Only the pinned `web_20260912` shared/conversation/composer bundles are admitted.
Their hashes and assertions live in `test-chatgpt-text-dispatch-public-evidence.cjs`.
They are parsed without executing downloaded code.

- Composer `DS` resolves to conversation `Jpt/thr`, which reads `readyFiles$` and
  calls `Ypt/nhr`. The latter is the exported synchronous ready-file formatter.
  It filters by model capability and project context, returning content and
  attachment metadata without making a request.
- Composer `SJ` passes the result into shared `CZ/mb`: string content becomes a
  text part; image content remains `multimodal_text`. Attachment metadata stays
  on the user message. `AB/eQt` preserves it in the actual request.
- Shared `Pm/YEt` uses `sediment://` for `file_` IDs and `file-service://` for the
  other observed IDs. Images retain exact dimensions and byte size.
- Composer `fun` lowercases and deduplicates selected MIME types; `FB` includes
  them in preparation. Actual ordinary dispatch does not copy that prepare-only
  field. The outgoing request preserves message follow-up support.

Binding v25 exposes only the reviewed `Ypt` export. The separate
`chatgpt_web_fresh_text_attachments.js` module validates the formatter output;
context v7, request v6, reconcile v5 and transaction v15 integrate the result.
Asset injection remains in `ChatGptWebAdapterAssets`, not the large page adapter.

## Ownership And Delivery

The selected files must come from the existing private composer's `prepareSubmit`
lease, not from arbitrary IDs in the DOM. The lease freezes metadata, preserves
actual File identity, and binds the account, document, conversation and store.
Admission requires all selected files ready, no upload in progress, unique IDs,
valid specifications and no more than the existing nine-file limit.

The provider's formatter retains document/image/library metadata. Every selected
file must appear in the resulting message. A model silently dropping a document,
wrong image pointer, changed metadata or selection prevents dispatch. Formatter
output is bounded and checked again against its preparation snapshot. File-only
requests may have an empty text part; an empty selection cannot authorize one.

On a successful HTTP/SSE response, cleanup consumes only the exact submitted
entries. Files added after dispatch remain selected. Cleanup is idempotent and
does not replace the entire store. A cleanup failure retains the completed
transaction until local cleanup succeeds; it never submits the message again.

If a response is lost after dispatch, files remain until the exact terminal
history proves the conversation, parent/user branch and all submitted references.
Both network-history and already-hydrated store checks require those references.
Missing files cannot be mistaken for successful attachment delivery merely
because the user-message ID exists. Recovery is read-only, never another POST.

There is no auth/Cookie export, new endpoint, automatic upload retry or WebView
reload. Known pre-dispatch compatibility gaps may use the accepted sender;
post-dispatch uncertainty cannot fall back to another writer.

## Verification And Remaining Work

- `fresh-attachment-verified-20260913-135120-884`: 408 Node tests passed,
  zero failures/skips. Includes pinned public contracts, versioned bindings,
  fresh ordinary/tool/project/new/temporary sends, private attachment selection
  and composer/lease, and the existing runtime attachment submission path.
- Integration tests compose the real ownership lease, context, request,
  transaction, stream follower, stream transport/projection and history reconciler
  with a synthetic provider.
  They check default-off/one-command trial, exact request metadata, duplicate
  command, later files, pre-dispatch selection change, lost response, missing
  history references and failed local cleanup. They are not live HTTP evidence.
- The first integration run used a synthetic command ID longer than the native
  Long/base36 ledger permits. The fixture was corrected; production admission
  was not relaxed. Only the passing run counts.
- Follow-up source audit found that transport v21 rejected empty text even with
  a valid file lease; the initial transaction tests mocked that transport away.
  Transport v22 now uses a one-use stream admission independent of the optimistic
  text bubble. Transaction v15 explicitly admits file-only sends only after the
  attachment lease was verified. Empty attachment-only sends never synthesize a
  blank bubble. Snapshot merging cannot consume the stream admission, and reset,
  finish and disposal invalidate it. The real transport replaces the mock in all
  attachment transaction tests. `fresh-stream-admission-20260913-141543-065`:
  12 Node tests passed, including the legacy transport suite. This remains offline
  evidence, not production file-only acceptance.

Enable only for grouped acceptance using the existing one-command
`fresh_text_trial_start`, or explicit `__elonChatGptFreshTextAttachmentsEnabled`.
The normal production attachment path is unchanged until acceptance succeeds.

Next grouped APK: attach controlled TXT/PDF/PNG files in the production native
composer, send one turn, verify actual file-content recognition, streaming,
selected-card cleanup, later-file preservation and exactly one server user turn.
Check file-only send and preserve the previous conversation/login. No APK or
device test was performed in this source batch.

Shared library references, mounted files and context connectors remain excluded
from this formatter wrapper and retain their existing paths. Ordinary library
metadata is supported in source but needs its own real sample. New, project,
temporary and tool combinations require their separate gates and grouped
acceptance; their prior runtime success is not independent HTTP acceptance.
Thermal measurements remain deferred until private functional acceptance.
