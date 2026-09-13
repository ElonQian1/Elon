# Library-Linked Writing Block Save

Capability: `android_chatgpt_writing_block_library_save_v1`.
Status: `partial / offline_verified / source_only`, not `completed`.
This extends the [existing native editor](chatgpt-writing-blocks-native.md),
not the standalone Library editor or Canvas. Adapter 384; parser 6,
policy 3, writing context 5, transport 3 and library session module 1.

## Implemented Scope

- Complete assistant Writing Blocks, including typed widgets, with one exact
  `libfile_`/`libfile-` identity in the original block or saved metadata.
  Conflicting or malformed identities cannot authorize a write.
- Existing ordinary/owned-project conversation guards still apply. Native
  editing, undo/redo, copy and export remain independent of cloud admission.
- Before saving, require the actual page-owned Library session to be hydrated
  from Library, clean, without pending editor changes or an in-flight save,
  and to match the server-confirmed conversation block content. Capture file
  ID, base version and save sequence; recheck them inside the official queue.
- Use `updateDraft(source: chat)` and `beginSave` on that same session, then
  the existing single message-scoped POST and authoritative readback. Only
  confirm the reserved sequence after exact content/ownership verification.
- A known HTTP rejection releases our reservation and restores only our own
  unchanged draft. Another editor's work is not rolled back. An uncertain POST
  remains reserved and is verifiable, never automatically resubmitted.
- Queue timeout cancels the pending callback so it cannot send later. The
  session retain is released after the operation; no polling loop is added.
- After confirmation, acknowledge the Library session, invalidate only its
  file-preview caches, and reuse the observable single-message tree update.
  No WebView reload or full conversation hydration is introduced.
- Library write-source admission requires explicit opt-in from transport v3.
  A resident v2 writer cannot gain uncoordinated Library write access when a
  newer parser/policy is injected; its existing uncertain receipts are retained.

## Protocol Evidence

Retained public JavaScript was parsed, not executed. The reviewed runtime is
`web_20260912`; unknown runtime symbols are not guessed.

| Asset | SHA-256 | Contract |
|---|---|---|
| `a965fc59-fzrm5l4zirdbhwph.js` | `752c85e9623229704c208167584c5b7a6e8f18410e6258713d2de7d483a62e19` | `nc`: linked writing uses `enqueueSave`, `beginSave`, completion and `POST /conversation/message/writing-blocks` |
| `conversation-small-h1dtzoris1y9588z.js` | `da08c64c132306779e09ba89cac64fa560b120e7560ffdc29b3ce5a0b8ccd67e` | `uDt -> xZn -> vZn`: page-owned singleton Library document sessions |
| `560eefd0-d3e3qb2j1xlsfaom.js` | `1c5ae9d0063fec37d5a0773e7c83aaf31cd13d717ee8ea21269cb5df48132d64` | `a -> f`: acknowledge saved content; invalidate file-preview and file-preview-contents queries |

The standalone Library PATCH and its `expected_current_version` are a different
contract. They must not be substituted into the chat Writing Block endpoint.
The chat endpoint's `updated_at` is not a server compare-and-swap condition.
Local coordination and read-check-write detect observed conflicts, but cannot
guarantee freedom from simultaneous cross-device writes.

## Verification and Remaining Work

`writing-library-regression-20260914-014647-088`: 190 Node tests passed,
zero failures/skips, 22.8 seconds. Includes retained-public-source hash/AST
checks, runtime bindings, actual parser/context/transport with a synthetic
Library store and HTTP, ordinary/project/widget regression, history and SSE.
No real user content or credentials are stored in fixtures or receipts.

No phone was connected during this batch. Android compilation, APK publication,
and linked-document native save/return/reopen acceptance remain deferred to the
grouped build. Offline success is not a live endpoint or device pass.

Missing or conversation-seeded-only Library sessions are still local-copy-only;
this batch does not invent authoritative Library hydration from stale content.
Live inferred-resource-only links, temporary/shared writing, standalone Library
document editing, and cross-device atomic writes remain outside this scope.
Next acceptance uses one owned, explicit linked document: native edit, one save,
matching conversation and Library content, reopen, and preservation of the
original selection. Reuse the already accepted ordinary editor/export cases.
