# Private attachment byte routes

Capability: `android_chatgpt_private_attachment_byte_routes_v1`.
Status: source implemented and offline verified on 2026-09-07; grouped APK and
device acceptance pending. Not `completed`. This extends the existing private
create/upload/process transaction; it does not replace native file selection,
image preparation, conversation binding or official ready-file association.

## Confirmed source contract

Public source fingerprints:

- [Composer upload implementation](https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js),
  SHA-256 `9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`.
- [Shared byte protocol](https://chatgpt.com/cdn/assets/conversation-small-hiw4wce20lu6te81.js),
  SHA-256 `296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`.

The first asset's `SGt` advertises `supports_direct_azure_multipart=true`.
`KWt` validates `kind=direct_azure_multipart`, `part_size_bytes`, `part_count`
and `max_part_concurrency`; the count must equal the ceiling of size/part size.
`VGt` selects `KGt` for that strategy, then starts processing only after commit.
The second asset's `Tqt`, `Eqt`, `Dqt`, `Oqt`, `kqt`, and `Aqt` define the
zero-based eight-digit Base64 block IDs, `comp=block` PUTs, ordered XML block
list, commit URL and Azure `2020-04-08` headers.

`VGt` separately selects `WGt` for the `upload_content_bytes` Estuary path.
`WGt` POSTs FormData with the file and `upload_url`, using the nested destination
query value when present, otherwise the returned upload address. It includes
same-origin credentials and lets FormData choose its multipart boundary. This
is followed by the same processing stream, not acceptance of the upload response
as an indexing or message receipt. These are source observations, not a new
successful live upload capture.

## Implementation

`chatgpt_web_private_attachment_bytes.js` owns byte-route validation and transfer.
The existing attachment transport selects it only when its version-1 module is
loaded, advertises multipart support and delegates the byte stage. Without the
module, the prior whole-file contract is unchanged. Production asset registration
loads the module before the attachment transport; no DOM click is added.

- Whole-file Azure/AWS PUT retains the existing signed destination and headers.
- Azure multipart validates the exact part geometry, caps it at 128 parts, and
  uses at most two workers, never exceeding the server's concurrency. `File.slice`
  supplies disjoint byte ranges; the original file-size limit stays 8 MiB. All
  successful parts precede one ordered commit and one processing request.
- Estuary is restricted to HTTPS `chatgpt.com` at the evidenced
  `/backend-api/estuary/upload_content_bytes` or `/api/estuary/upload_content_bytes`
  path. A nested destination must pass the existing signed-blob validation.
  Unknown paths, external forwarding targets, custom upload headers and mixed
  strategies are rejected. No supplied page Content-Type overrides FormData.
- Page authorization/workspace headers go only to the official origin, never to
  blob parts or commit. Cookies, signed URLs and server error bodies are not
  exposed through the native completion receipt.
- The byte stage has one 30-second deadline across all parts and commit, not a
  fresh allowance per part. Every request uses the existing bounded helper and
  current-document/account/conversation owner. First failure aborts sibling
  requests; cancellation, context drift or any failed part forbids commit and
  processing. There is no automatic retry, reload or alternate upload replay.

Creation/PDF model headers and processing privacy/project metadata are unchanged.
Only the existing final processing and exact composer association can make an
attachment ready to send. This is page-local private HTTP with WebView identity;
it is not a fully independent Android networking implementation.

## Verification and remaining work

148 tests passed with no failures or skips across the byte-route, attachment
transport, composer, thread, image, project, read-only protocol, document and
native-source Node suites. Checks use synthetic Fetch responses and the actual
bounded request helper. They cover byte integrity, ordered commit, concurrency,
credential separation, hanging-sibling cancellation, total deadline, no replay,
production asset syntax/registration and unchanged scope/MIME receipts.

No Android build, installation, microphone or live upload ran in this batch.
The previous small-text private-upload success does not verify these new routes.
During the grouped APK round, retain the actual selected-route requests and
native association receipt, then verify a reply reads the fixture exactly once.
If the server does not select a route, record that case as unobserved rather
than manufacturing a production response or calling it verified.

Reservation prefetch/claim is now [integrated in source](chatgpt-private-upload-reservations.md),
overlapping byte preparation without waiting; picker-open prewarm, temporary
reservation persistence, direct-library reuse and remaining file categories
are still separate gaps. Existing [native integration](chatgpt-private-attachment-upload.md)
and [scope rules](chatgpt-private-attachment-scopes.md) are reused. Do not repeat
protocol implementation merely because live acceptance remains pending. Actual
latency, energy and thermal improvement has not been measured.
