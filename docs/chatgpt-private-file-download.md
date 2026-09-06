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

`chatgpt_web_private_file_download.js` owns a bounded, two-minute in-memory
selection registry and one active authorization request. It registers only
ordinary conversation attachment descriptors with an explicit `file-*` ID.
Project, shared-library, library-linked, connector and image-pointer variants
remain unclaimed until their extra scope-resolution contracts are covered.

The native index receives random opaque selection handles, not private file
IDs or signed URLs. Registration uses the selected history branch, preserving
the existing message/attachment order. Refresh invalidates old selections.
Account, document and route changes prevent a late authorization response from
starting another download. No page navigation, composer edit, upload or send
is needed. Unknown or expired selections ask for a list refresh, not a DOM scan.

The native handoff receives the signed URL only through a one-use download
lease. It must independently validate HTTPS, the `*.oaiusercontent.com` origin,
the originating main frame and document generation. Cookie and authorization
headers are never handed to the Android download service. The signed URL is
necessarily held by that service for the requested transfer, not exported in
MCP responses or stored in conversation caches. Android's download service owns
transfer progress, storage and completion notifications. `download_queued`
means enqueued, not downloaded or saved successfully. Failure or an unconfirmed
acknowledgement is never automatically replayed.

## Delivery

Protocol module and eight targeted JavaScript checks pass. Native integration,
grouped APK and actual synthetic-file download acceptance are pending. No
latency, battery or temperature improvement is claimed without a device sample.
