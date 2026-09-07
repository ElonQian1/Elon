# Shared-library attachment downloads

Capability candidate: `android_chatgpt_private_shared_library_download_v1`.
Status: **implemented source candidate, offline JS verified, native build and
grouped device acceptance pending**. This is not a `completed` capability.
Adapter 281, file-download module 5, library-download module 1.

## Official source evidence

The 2026-09-06 public assets were rechecked by SHA-256 on 2026-09-07:

- `/cdn/assets/4813494d-hrplraurzfyvxb10.js`, SHA-256
  `89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0`:
  `AX` constructs `/api/library/files/{id}/download`. `fEt` uses that route for
  `sharedLibraryFileId` or `libraryDownloadId` by clicking a download anchor.
  It does not use the ordinary `/backend-api/files/download/{id}` JSON resolver.
- `/cdn/assets/conversation-small-hiw4wce20lu6te81.js`, SHA-256
  `296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`:
  `Gar` serializes a shared-library reference with `library_file_id`, `name`,
  `source: library`, optional MIME/size and no ordinary file ID. The preview
  resolver also recognizes matching file/library IDs, under an official flag
  whose current runtime binding remains unverified.
- `/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js`, SHA-256
  `9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`:
  `attachSharedLibraryFileReference` represents that reference in the ready file
  spec using the library ID; it is not a newly uploaded ordinary file.

These observations establish a source contract, not a successful authenticated
GET, redirect/CORS behavior, saved-file transfer or runtime feature-flag result.

## Bounded native integration

The existing production conversation-file Download action now recognizes the
selected history attachment when its source is `library`, its explicit ID is a
recognized `libfile_`/`libfile-` value, and its ordinary ID is absent or matches.
Existing project-conflict, extra-scope, identity, route and document checks stay
in force. Opaque handles snapshot the target; mutable history cannot retarget it.

The identity WebView fetches the binary route with same-origin cookies. No
Cookie or bearer header is copied to Android HTTP. Final response URLs must be
the requested route or an already-allowed signed CDN URL. HTML/login responses,
unexpected JSON, truncated bodies and oversize files fail explicitly. The real
route's redirect and CORS compatibility must still be checked in the browser or
phone WebView; there is no bypass or alternate guessed request on failure.

Bytes pass through the existing one-use native lease in ordered 48 KiB packets,
with one packet awaiting acknowledgement. A single I/O worker decodes and writes
the stream. The 512 MiB limit and 120-second total deadline bound the transfer;
headers and each native acknowledgement have an 8-second deadline, body reads a
20-second idle deadline. Compressed wire lengths are not mistaken for decoded
file lengths. Native command tracking allows 135 seconds for terminal reporting.

Android 10+ uses a pending MediaStore Downloads row, published only after EOF
and byte-count checks. Android 8/9 uses a unique app-external Downloads `.part`
file and renames it on completion. Ordinary cancellation/failure discards the
incomplete output. File writes and cleanup do not run on the UI thread.
`download_saved` means native publication succeeded; ordinary signed-URL
downloads retain their separate `download_queued` receipt. A lost final receipt
is indeterminate, never an instruction to replay or create a duplicate file.

Existing notification permission enables progress at most once per second,
cancel, and opening the saved local URI. No new permission prompt is introduced.
Native cancellation aborts the page stream. Navigation and disposal revoke the
lease; private identifiers, URLs and file contents stay out of MCP receipts.

## Exact remaining gaps

- Standalone `libraryDownloadId`, mounted-library, connector-only cloud and
  parameterized-image references are not implemented by this candidate.
- References present only in separate shared-library metadata/citation graphs,
  rather than the selected conversation attachment index, are not yet indexed.
- Process-death resume and startup cleanup of orphaned pending rows/part files
  are not implemented. Normal cancellation cleanup is not crash recovery.
- When notifications are disabled, there is not yet a dedicated in-app
  progress/cancel control. The existing file sheet gets a terminal result, and
  page-context revocation cancels the transfer. This is a UI gap, not a pass.

## Verification and grouped acceptance

The final combined run passed **147 Node cases**, zero failures or skips. It
covers the new binary stream plus ordinary/image/connector downloads, history,
file index, attachment composer, private transport, gallery and production asset
bundle parsing. HTTP, binary content and native acknowledgements are synthetic.
Cases include ordering, limits, immutable targets, late identity/route changes,
native cancel and lost commit acknowledgement without replay.

Five new pure Kotlin transfer tests and one command-lifecycle test are written
but not executed yet. No Android build, APK publication or phone file transfer
occurred in this source batch. No speed, heat or battery improvement is claimed.

In the grouped ChatGPT round, select a synthetic shared-library attachment from
the production native file sheet and verify actual saved bytes/MIME and route
provenance. Check cancel during transfer, insufficient storage, notification
permission, supported Android storage paths and real VPN/redirect behavior.
Preserve the original draft, conversation and voice session. Test other pending
ChatGPT candidates in the same APK; Google remains after that acceptance gate.
