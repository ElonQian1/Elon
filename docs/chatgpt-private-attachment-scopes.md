# ChatGPT private attachment scopes

Capability: `android_chatgpt_private_temporary_attachment_upload_v1`.

Status: **implemented, offline verified, grouped Android and device acceptance
pending**. Adapter 271 extends the existing private text/static-image upload into
temporary chats. It is not a released APK, a real temporary-file transaction, or
proof of backend retention behavior. Project uploads remain a separate code gap.

## Current official contract

The following public assets were inspected on 2026-09-06; the full upload
transaction is described in [the attachment contract](chatgpt-private-attachment-upload.md).

- `4813494d-hrplraurzfyvxb10.js`, SHA-256
  `89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0`:
  `Xv`, exported as `cX`, reads `temporary-chat=true` from the current URL.
- `8b34dbc2-kjj15hg4y6iyx13p.js`, SHA-256
  `9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`:
  imports that signal as `Vp`. `X$n` feeds the composer's `Br`; temporary mode
  makes `Vr` false, library-enabled `Hr` false and persistence mode `Ur` absent.
  `KJ` carries those options to `gB.uploadFile`. `SGt` omits an undefined
  `library_persistence_mode`; `TGt` puts `is_temporary_chat` and `store_in_library`
  in processing metadata. There is no separate temporary-upload endpoint.
- `conversation-small-hiw4wce20lu6te81.js`, SHA-256
  `296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`:
  `jRr`, imported above as `eVe`, preserves explicit `storeInLibrary=false`
  rather than replacing it with the account's library default.

These are source-level request contracts. They do not prove that every account
has the same enabled upload transport or permissions. The legacy upload route
continues to reject unknown signed destinations, custom upload headers and
multipart/reservation response shapes before any byte dispatch to those routes.

## Production integration

The existing attachment composer accepts ordinary new/existing conversation
paths with either no query or exactly one `temporary-chat=true` query. Other
query combinations, duplicate mode keys, project routes and unrelated surfaces
do not become temporary uploads by inference. No DOM label or cached selection
is used to invent the mode.

New chats bind the current URL, document, identity, model and exact official
file store. Existing chats additionally use the current no-store conversation
read: the conversation ID must match, `is_do_not_remember` must explicitly agree
with the requested mode, and project/scoped contexts are excluded. A present
`is_temporary_chat` must be boolean and cannot contradict the canonical flag.
Unknown scope chooses the existing compatible route before byte reads or writes.

Temporary uploads explicitly send `store_in_library=false`, no library
persistence mode and processing metadata `is_temporary_chat=true`. They keep
retrieval indexing disabled. The private receipt retains the mode, and ready-file
association checks it against the captured binding before updating the official
store. This applies to text and normalized static images using the existing
image preparation, upload and native send owner.

An explicit `library_persistence_result=library` contradicts temporary intent:
the operation fails confirmation instead of reporting a ready attachment or
replaying a write. It does not automatically delete an unknown server artifact.
Server-side retention, absence from the library and cleanup still require live
acceptance. Mode, conversation or account changes during upload cannot attach
the result to the new context. The existing draft and text sender are not replaced.

## Project checkpoint

Project attachments cannot safely be enabled by changing `isProjectThread` alone:

- `BQr`/`HQr` in the conversation asset checks
  `gizmo.current_user_permission.can_write`; loading permissions prevent upload.
  Read-only members receive chat-only attachment behavior, not project writes.
- `WQr` includes the originating thread/leaf and, when writable,
  `gizmo_id`, `is_project` and `should_upload_to_project` in library metadata.
- `BZt` converts a normal file to the `gizmo` use case with the confirmed project
  ID, but keeps a project image as `multimodal`. `wGt` in the upload asset also
  derives retrieval indexing from `projectUsesInjestPath` and file type.

Next connect current permission/identity and originating leaf evidence to these
options, then the create/process bodies and ready-store association. Do not
infer write access from a sidebar project title, use a stale directory cache,
or claim project completion from temporary/ordinary tests.

## Verification

The focused Node suites cover temporary text and image association, ordinary
regressions, the actual private conversation reader with synthetic HTTP,
conflicting scope fields, duplicate query keys, mode changes during upload,
privacy metadata, contradictory processing receipts and no automatic replay.
The initial new tests failed on the unchanged ordinary-only implementation;
the implementation and updated version assertions now pass 67 runner cases.

No Android compilation or device test is claimed here. The grouped acceptance
must use a synthetic file/image in new and existing temporary conversations,
confirm one upload and one requested reply, verify no new personal-library entry,
then check cancellation and switching back to ordinary chat. This does not
require repeating completed audio, subtitles, dictation or read-aloud research.
