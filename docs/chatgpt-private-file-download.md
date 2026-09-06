# Private conversation file downloads

## Contract evidence

The 2026-09-06 official asset
`https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js`
has SHA-256 `89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0`.
Its `dEt` download resolver uses `GET /files/download/{file_id}` under the
existing backend API base, with conversation/context-scope and download-intent
query parameters. Success returns `download_url`; `retry` is not success.
The importing conversation asset's `C4t` passes `conversation_id` when there
is no alternative context-scope argument. This is observed official source,
not a guessed endpoint or a device download acceptance result.

## Scope

Capability candidate: `android_chatgpt_private_file_download_v1`.
Status: `implemented_device_pending`, not `completed`.

`chatgpt_web_private_file_download.js` owns a bounded, two-minute in-memory
selection registry and one active authorization request. It registers only
ordinary conversation attachment descriptors with an explicit simple file ID
from the selected history response; it does not assume a particular ID prefix.
Module version 2 additionally registers confirmed project and library-linked
attachment descriptors as described below. Version 3 adds scoped
[conversation image-pointer downloads](chatgpt-private-image-download.md) and
rejects attachment-level context scopes and `library_download_id` lanes.
Shared-library, connector and parameterized image-pointer variants remain
unclaimed until their separate resolvers are covered.

The native index receives random opaque selection handles, not private file
IDs or signed URLs. Registration uses the selected history branch, preserving
the existing message/attachment order. Refresh invalidates old selections.
Account, document and route changes prevent a late authorization response from
starting another download. No page navigation, composer edit, upload or send
is needed. Unknown or expired selections ask for a list refresh, not a DOM scan.

The native handoff receives the signed URL only through a one-use download
lease. `ChatGptWebFileDownloadGateway` independently validates HTTPS, the `*.oaiusercontent.com` origin,
the originating main frame and document generation. Cookie and authorization
headers are never handed to the Android download service. The signed URL is
necessarily held by that service for the requested transfer, not exported in
MCP responses or stored in conversation caches. Android's download service owns
transfer progress, storage and completion notifications. `download_queued`
means enqueued, not downloaded or saved successfully. Failure or an unconfirmed
acknowledgement is never automatically replayed.

The existing production conversation-file detail dialog has a Download action
for registered descriptors. The consumer and MCP ports use the same tracked
command and reject a selection handle that no longer matches the native index.
Failure releases the pending lease immediately; retry never waits for its TTL.
The list and original-conversation actions remain available even if optional
download registration fails. Android 10+ saves to public Downloads; Android 8/9
uses the app's external Downloads directory without requesting broad storage
permission. Filenames are sanitized and prefixed with a unique lease ID, so
parallel downloads cannot overwrite each other.

## Delivery

Protocol module and native integration are implemented in adapter 266 / private
transport 20. Nine download checks, 15 file-index checks, 12 history-projection
checks, 15 attachment integration checks and the private-transport regression
suite pass. Release source compilation and 22 focused Android tests pass.
Grouped APK and actual synthetic-file download acceptance are pending. No
latency, battery or temperature improvement is claimed without a device sample.

Grouped acceptance should reuse the existing synthetic 78-byte attachment,
download it from its native conversation-file detail action, verify the saved
bytes rather than only the queue receipt, and preserve the original draft,
conversation and voice state. Also verify transfer through the device's actual
VPN configuration because Android's download service is a separate process.

## Project and library extension

Capability candidate: `android_chatgpt_private_scoped_file_download_v1`.
Status: **implemented, offline verified, grouped APK/device acceptance pending**.
This extends the same production file-list Download action and one-use native
lease; it does not add another UI or substitute a copied conversation URL.

Further inspection of the same SHA-256-verified shared asset establishes:

- `WTt` requests `GET /files/{file_id}/simple` with `gizmo_id` and
  `conversation_id` under `/backend-api`.
- `KTt` resolves file metadata when a library file is present; `DX` selects the
  effective project from that metadata. A personal library file removes the
  requested project, while a project library file uses its returned project ID
  or the requested project when the response does not specify one.
- `fEt` passes that effective project and
  `checkContextScopesForConversationId` into `dEt`, with `downloadIntent=true`.
  `dEt` emits `gizmo_id`, `check_context_scopes_for_conversation_id` and
  `download_intent` query parameters. It returns a signed download URL, not
  file bytes or an authenticated Android request.
- The conversation asset's library preview also reads `fileInfo.library_file_id`
  and `is_library_file`. Its `YB` recognizes `libfile_` and `libfile-` identifiers.
  Shared-library content and standalone library downloads have distinct routes;
  they are not substituted for conversation attachment downloads here.

The version 2 candidate accepts `/c/<id>` and
`/g/g-p-<32 hex>[-slug]/c/<id>`. Project IDs in the route, selected conversation
and attachment must be canonical and agree. Extra context scopes, conflicting
projects, unknown library IDs, shared-library fields, connectors and image
pointer variants did not receive a private download handle in version 2; the
simple image-pointer extension is described above. Registration snapshots the
IDs, so a mutable history payload cannot retarget an existing selection.

Ordinary non-library files retain the original one-request contract. Project
files use the selected conversation's context-scope query. Library-linked files
first make a fresh, bounded simple-metadata read; the response must confirm the
same library file, correctly typed scope fields and any returned file ID.
Metadata may resolve a different actual project or remove the project for a
personal-library file, as in `DX`; the final authorization still carries the
selected conversation's context-scope check. Unknown metadata fails without
trying a broader ordinary-file request. This is intentionally not the official
helper's catch-and-reuse behavior when its metadata read fails.

Metadata has a 6-second deadline and 64 KiB limit; authorization retains its
8-second limit and the entire operation's 15-second deadline. The existing
account, document, route, cancellation and single-flight guards span both reads.
Only the confirmed signed URL enters the existing one-use Android handoff.
No file-info response, Cookie or authorization header reaches the native index
or download service, and no signed link is persisted in history caches.

The download extension and existing history, file-index, private-transport and
production asset-bundle tests pass 48 Node runner cases. New cases were first
run against the unchanged implementation and failed because project/library
downloads had no handles. Tests use synthetic HTTP and cover both route forms,
personal/project library resolution, malformed/conflicting metadata, mutation
after selection, bounded reads and late cancellation. No Android compilation,
APK publication or device transfer occurred in this batch; the handset was away.
The adapter header remains 272 and the download module is version 2.

Grouped acceptance must download one synthetic project attachment and one
library-linked conversation attachment from the native file sheet, confirm the
saved bytes and correct account/project scope, and preserve the current draft,
voice and conversation. A queue acknowledgement is still not a saved-file pass.
