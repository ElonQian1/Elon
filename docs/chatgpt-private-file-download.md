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
Project, shared-library, library-linked, connector and image-pointer variants
remain unclaimed until their extra scope-resolution contracts are covered.

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
