# ChatGPT private attachment scopes

Capability: `android_chatgpt_private_temporary_attachment_upload_v1`.

Status: **implemented, offline verified, grouped Android and device acceptance
pending**. Adapter 271 extends the existing private text/static-image upload into
temporary chats. It is not a released APK, a real temporary-file transaction, or
proof of backend retention behavior. Adapter 272 adds the separately scoped
new-project candidate below. The subsequent JavaScript-only batch adds existing
project branch binding; actual runtime access and project upload acceptance
remain pending, rather than being inferred from public source inspection.

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

Capability: `android_chatgpt_private_new_project_attachment_upload_v1`.
Status: **implemented, offline verified, grouped Android/device acceptance pending**.
It is deliberately not a completed capability or a claim about every project scope.

Project attachments cannot safely be enabled by changing `isProjectThread` alone:

- `BQr`/`HQr` in the conversation asset checks
  `gizmo.current_user_permission.can_write`; loading permissions prevent upload.
  Read-only members receive chat-only attachment behavior, not project writes.
- `WQr` includes the originating thread/leaf and, when writable,
  `gizmo_id`, `is_project` and `should_upload_to_project` in library metadata.
- `BZt` converts a normal file to the `gizmo` use case with the confirmed project
  ID, but keeps a project image as `multimodal`. `wGt` in the upload asset also
  derives retrieval indexing from `projectUsesInjestPath` and file type.

The same shared asset's `xR.getGizmo` reads `/gizmos/{gizmo_id_or_short_url}`
through the authenticated backend client (`https://chatgpt.com/backend-api`).
The project's `mZ`/`qDt` store exposes that response as `gizmo$()`; permission is
`gizmo.current_user_permission.can_write`, not a conversation-directory title.
The upload asset imports shared `EEt` as `Cie` and `TEt` as `gg`: the former tests
`xls/xlsx/csv` suffixes; the latter tests image suffixes. With `use_injest_path`,
spreadsheet suffixes select retrieval, images depend on flag `2031707412`, and
other files do not select retrieval. This corrects any assumption that all
project text uploads require retrieval indexing.

The candidate connects `chatgpt_web_private_attachment_project.js` to the existing
composer, native byte lease, upload transport and ready-file association. It accepts
only an empty new-project composer at `/g/g-p-<32 hex>[-slug]/project`, without query
parameters. An explicit user upload reads fresh project permission with `no-store`,
a 7-second deadline, a 1 MiB body limit and cancellation. Project ID and boolean
permission must match. Unknown permission retains the previous compatible route
before bytes or writes; account, document and URL changes fail without replay.
The initial candidate excluded read-only permission; the chat-only extension
below now supports that explicit permission without writing project files.
No permission polling or cached sidebar authorization is introduced.

Plain text uses `use_case=gizmo` with that project ID. Non-ingest static images
keep `multimodal` and dimensions without putting a gizmo ID in the top-level
create/process fields. Both put the exact confirmed project into
`metadata.library_file_info` with `is_project=true` and
`should_upload_to_project=true`. They do not invent an originating thread or leaf
for a new conversation. Personal-library preference stays explicitly false.
The existing proven legacy `required` persistence policy is retained as a candidate
combination, not claimed to have passed a real project upload.

The transport snapshots nested project metadata before authentication; processing
and ready-store receipts must retain the same project. Official `RGt` maps
`extra.metadata_object_id` to `libraryFileId`; `uploadCompleted` retains it on the
ready file and file specification unless persistence is temporary. The candidate
now preserves this mapping rather than losing it at native association.

The original candidate excluded read-only project dispatch and ingest-project
images. Both source extensions are described below; their live acceptance still
remains. Do not request project writes from a cached directory or mark all
project attachments complete from source tests.

## Ingest project images

Capability: `android_chatgpt_private_ingest_project_image_upload_v1`.
Implementation: **implemented**. Verification: **offline verified, device pending**.
Delivery: **source-only for the grouped APK**, not completed or installed.

The same inspected upload asset's `wGt` uses spreadsheet suffixes first, then
image suffixes and official gate `2031707412` when `projectUsesInjestPath` is true.
Shared `TEt` uses a case-insensitive suffix list; it does not classify by MIME
alone. Shared `cs`, exported as `t6`, exposes the current configuration client.
No new endpoint or copied credentials are introduced.

The project helper reuses its already-loaded, exact-version official module for
both selected-branch binding and configuration. After fresh write permission,
an ingest-image upload accepts the gate only when the client is Ready and the
gate has the exact name, boolean value, recognized evaluation reason and no
warnings. A recognized false value is supported, not a missing capability.
Unknown/loading/missing configurations retain the compatible path before native
byte reads or upload writes. The module wait remains bounded to 1.5 seconds.

New and existing project images retain `use_case=multimodal`, dimensions and
exact project/branch library metadata, without a top-level gizmo ID. The process
request's `index_for_retrieval` follows the confirmed flag. Indexed images still
require a validated project scope; ordinary and temporary images cannot opt in.
Each explicit upload reads the current local gate value, but reuses the module
promise. Text/PDF and non-ingest images do not wait for this unrelated gate.
Account, route, document, selected-branch and cancellation guards remain active.

Project module version 4, composer 8 and protocol 7 pass **101 targeted Node
cases** across seven attachment suites, including production asset-bundle parsing.
The new feature cases first failed against the unchanged implementation and then
passed. This round did not run Gradle, package/publish/install an APK, access the
handset or prove a live indexed-image upload. Grouped acceptance must confirm
the actual account gate/module and one synthetic image's intended project
association and usable reply without duplicate dispatch.

## Read-only project chats

Capability: `android_chatgpt_private_readonly_project_attachment_upload_v1`.
Implementation: **implemented**. Verification: **offline verified, device pending**.
Delivery: **source-only for the grouped APK**, not completed or installed.

The inspected `BQr`/`HQr` contract above returns `chat-only`,
`canStartUpload=true` and no upload project ID for an explicit
`can_write=false`. It does not reject all attachments. The upload asset imports
`WQr` as `obe`: its production composer passes the permission-filtered project
ID plus the current server thread and selected leaf. Consequently `WQr` emits
only the paired conversation origins for an existing read-only chat, and no
library-file metadata for an empty new one. `TGt` still carries
`is_project_thread=true`. `BZt` does not convert the file use case to `gizmo`
without an upload project ID. These are source-level contracts, not a live
permission or persistence acceptance result.

The production native upload path now distinguishes explicit writable and
read-only scopes. Missing/malformed permission remains unknown and cannot default
to either. Text/PDF chat-only uploads use the existing `ace_upload` transaction;
normalized static images keep `multimodal` and the confirmed ingest-image gate.
Neither create nor processing requests carries `gizmo_id`, `is_project` or
`should_upload_to_project` for the chat-only branch. Existing chat origins stay
bound to the selected leaf; a new chat does not invent a thread or leaf.

`projectScopeId` is local transaction ownership, never a serialized request field.
The private receipt retains that ID and `projectWriteRequested`, which describes
the requested write scope, not independent proof of server file placement.
Association checks both against the captured permission. The official ready-file
store keeps `isProjectThread=true` but no `projectGizmoId` in the chat-only branch;
it receives only the same optional origin metadata. This prevents native
association from accidentally upgrading a chat-only upload into a project write.
All existing account, document, model, selected-branch, cancellation, deadline
and no-ambiguous-replay guards are reused. A subsequent upload rereads permission,
so changing from writable to read-only changes the next request's target fields.

Project version 5, composer 9, protocol 8 and transport 6 pass **110 targeted Node
cases** across eight attachment suites. Production-path tests cover new/existing
read-only text, image and PDF uploads, exact create/process/ready-store fields,
receipt-scope mismatch, local removal and cancellation/identity/branch changes.
The new production cases first failed on the prior implementation. The full
production asset bundle also parses. No Gradle, APK publication, installation or
phone operation was performed in this source batch. Grouped device acceptance
must confirm actual read-only membership, one synthetic attachment/reply and
absence of a new project file; library placement/retention is not inferred from
the request fields or synthetic responses.

## Existing project branches

Capability: `android_chatgpt_private_existing_project_attachment_upload_v1`.
Status: **implemented, offline verified, runtime/device acceptance pending**.
It is not marked completed. No new upload endpoint or native send owner is added.

The same inspected upload asset imports shared `XM` as `Ad` and `HM` as `zr`.
Shared `sj`, exported as `XM`, resolves an official thread by server ID (including
the existing new-thread ID mapping). Shared `Z`, exported as `HM`, provides
`getGizmoId`, `getCurrentLeafId` and `hasNode`. The upload caller uses these
selectors and passes the chosen leaf and server conversation ID to `WQr`,
exported from the conversation asset as `H$`. `WQr` adds paired
`origination_thread_id` and `origination_message_id` to the same project library
metadata; `TGt` retains both fields in processing. The server's `current_node`
is not evidence of the user's selected UI branch.

The project helper binds only that exact, already-loaded shared module by dynamic
import. A matching resource-timing entry or modulepreload is required first; it
does not import guessed future asset names or create a copied official store.
One page-local promise is reused, with a 1.5-second wait deadline. Unknown module
exports, a loading/temporary thread, absent selected node or mismatched project
leave the private path unconfirmed before any upload write.

Existing `/c/<uuid>` and `/g/g-p-<32 hex>[-slug]/c/<uuid>` composers first read fresh
conversation membership. Only bounded UUID node IDs are exposed by this read,
not message bodies or the mapping itself. The official selected leaf must exist
in that response, and fresh project write permission must also succeed. This
candidate supports the same plain-text and non-ingest static-image subset as
new-project uploads. Both processing metadata and the official ready-file store
receive the same project, conversation and selected-leaf origin.

After branch capture, the guard stays active during permission reads, native byte
reads and upload processing. Changing branch, project, account, document or model
cancels the old operation, including when the permission request also fails;
there is no write replay through compatibility. Successful upload cleanup does
not invalidate the already-associated file merely because its job signal is
aborted. The current branch is captured after the fresh conversation read, not
claimed to have been captured at the initial file-selection button press.

Module versions are project `2`, protocol `5`, conversation transport `23`, and
attachment composer/send `6`. The Kotlin adapter header remains `272`; the grouped
APK must carry these newer assets. This source batch neither builds an APK nor
proves that the inspected module is loaded and accessible on the handset.

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

Adapter 272 passes 75 focused Node runner cases, including the actual bounded
JSON reader with synthetic responses, new-project create/PUT/process/association,
permission revocation between uploads, non-ingest image handling, scope changes,
nested metadata snapshots and exact library-file metadata. The production asset
dependency chain parses as one bundle. No Android source compilation or device
upload is implied. Grouped acceptance must additionally verify one synthetic
project file is usable in its intended conversation and appears in the correct
project, with no unintended personal-library entry or duplicate message.

The existing-project extension passes 86 focused Node runner cases. These use
synthetic official selectors and HTTP responses, and cover both route forms,
selected versus server-latest leaf, missing nodes, stale branch cancellation,
permission failure plus branch change, paired origin validation, timeout/abort,
text/image association and ordinary/temporary regressions. The actual production
asset dependency bundle also parses. This is not actual WebView module-import or
real project-upload acceptance. When the handset returns, verify one existing
project conversation with a synthetic attachment through production UI/MCP,
including final reply use, exact project placement and no duplicate dispatch.
