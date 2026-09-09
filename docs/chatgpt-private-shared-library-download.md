# Shared-library attachment downloads

Capability candidate: `android_chatgpt_private_shared_library_download_v1`.
Status: **implemented; ordinary library saved-byte acceptance passed in normal
Release 1.1.1584 with module 8**. The personal-library rendered TXT/PNG/PDF download
scope additionally passed on [normal 1612](reports/chatgpt-library-download-ui-20260909.md).
Additional shared/mounted scopes remain separate, not a blanket `completed` claim.
Current source: file-download module 14 and library-download module 8. The
existing adapter asset list is reused unchanged. The observed 302 to same-origin
estuary content had been incorrectly rejected; module 8 reuses the established
content-source validator. Sign-in is not a prerequisite to repeat.
See [current evidence](chatgpt-private-library-browser.md#1584-normal-release-acceptance).
The [1549 release evidence](reports/chatgpt-runtime-release-1549.md) is historical.

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
the requested route, the strictly validated same-origin estuary content route,
or an already-allowed signed CDN URL. HTML/login responses,
unexpected JSON, truncated bodies and oversize files fail explicitly. The real
library-to-estuary 302/200 transfer has been observed in the phone WebView;
other redirect/CORS variants remain unverified. There is no bypass or alternate
guessed request on failure, and the returned body is consumed without a second GET.

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

## Native progress and cancellation

The production conversation-file Download action now opens an in-app progress
dialog, independent of notification permission. Collapsing/dismissing it stops
only its one-second UI poll; the existing native worker continues. The file
sheet's Download progress action reopens the same request, without another GET.
The dialog distinguishes preparation, transfer, save, cancellation, saved,
system-queued, failed and unconfirmed states. A queued DownloadManager request
is not represented as a completed native save.

Consumer UI and MCP use the same request-bound cancellation command. A stale
panel cannot cancel the next transfer. Cancellation aborts our page-local fetch
owner and waits for native cleanup; repeated page aborts cannot finish an
in-flight save early. Successful publication wins a simultaneous cancellation.
Progress receipts expose only request ID, phase, byte counts and cancelability,
not the private lease, file name, URL, headers or contents. No official DOM
control or complete page snapshot is polled for each progress update.

## Exact remaining gaps

- Personal `libraryDownloadId` resolution for a conversation attachment is now
  implemented below. [Concrete mounted provider files](chatgpt-private-mounted-file-download.md)
  and Drive Docs/Sheets/Slides exports now have source implementation; standalone
  browsing, other provider-native exports, citation-only cloud and remaining
  image-pointer scopes/path forms are still not covered.
  [Bounded parameterized image pointers](chatgpt-private-image-download.md#parameterized-pointers)
  are implemented separately in the same download owner.
- Separate `metadata.shared_library_file_references` are now indexed as described
  below. Citation-graph-only references without that metadata remain unimplemented.
- Startup cleanup now has [durable native ownership and focused crash tests](chatgpt-private-download-recovery.md).
  Actual Android storage acceptance is pending. Process-death byte-range resume,
  persistent download history and old unjournaled artifacts remain unsupported;
  cleanup is not a resume or a restored completion receipt.
- Native controls are compiled into normal releases. Rendered small-file Download,
  saved status and Close passed on 1612. In-flight progress/cancellation and
  notifications-disabled behavior remain unaccepted; do not infer them from a
  transfer that completed before its first status inspection.

## Metadata-only shared references

The 2026-09-07 source extension reuses this capability, not another downloader.
The retained public assets above were hash-checked again; no authenticated
request or device result is inferred from reading them.

`attachSharedLibraryFileReference` in the `8b34dbc2` asset constructs
`{library_file_id, name, display_path, mime_type, size_bytes, entrypoint}`.
The composer sends those ready-file references through `EOn`; `zVr` in the
conversation asset places them in `metadata.shared_library_file_references`,
separately from ordinary attachments. The message reference renderer looks up
the library ID there and passes it to `dA` as `sharedLibraryFileId`. `j$n`
passes that preview target to `fEt`, which uses the existing `AX` binary route.
This establishes the descriptor-to-download mapping, not merely a matching URL.

History projection now appends these files after existing image/attachment
parts, keeping their previous row positions and the selected regeneration
branch. Ordinary attachments take precedence when the same library ID appears
in both collections; duplicate shared IDs do not create repeated rows. The
per-message 20-reference, 100-index-row and 80-displayed-message bounds remain;
an older shared file can still appear in the conversation file sheet. A bounded
index reports truncation instead of claiming to be complete.

Only the download owner receives the raw reference, as a distinct descriptor.
It admits the evidenced fields, a valid library ID/name and typed optional
MIME/size; alternate IDs, scopes, preview targets and unknown fields cannot
fall through to the ordinary file resolver. Identity/project/document guards,
opaque selection handles, cancellation, byte ordering and native publication
reuse the existing implementation. Display paths, IDs and entrypoints are not
exported to native display rows. No additional metadata GET, DOM query, new
native UI or second transfer owner was added. Module reinjection retires the
old download registry once without restarting the identity transport.

The new projection cases failed on the unchanged source. The final focused
runs pass **149 Node runner cases**, including shared/ordinary/connector/image
downloads, history, file index, identity transport and production asset-bundle
parsing. Two synthetic scope variants verify exact saved bytes and immutable
selection, with cancellation/identity loss preventing publication. HTTP and
native acknowledgements are simulated. Android compilation, actual shared
metadata, binary-route redirect/CORS and device saved-file acceptance remain
pending for the grouped build; this is still `implemented_device_pending`.

## Verification and grouped acceptance

The final combined run passed **149 Node cases**, zero failures or skips. It
covers the new binary stream plus ordinary/image/connector downloads, history,
file index, attachment composer, private transport, gallery and production asset
bundle parsing. HTTP, binary content and native acknowledgements are synthetic.
Cases include ordering, limits, immutable targets, late identity/route changes,
native cancel and lost commit acknowledgement without replay. The two new
request-bound cancellation cases fail against the original module at
`17e41589c` and pass against module 6; three existing matching cases pass on both.

The five pure Kotlin transfer cases and six download-session cases subsequently
passed as part of the 31-case [crash-cleanup checkpoint](chatgpt-private-download-recovery.md).
The broader command-lifecycle, consumer/MCP and UI-contract tests still await
grouped Android execution. The UI-contract test checks source wiring, not rendered pixels.
No Android build, APK publication or phone file transfer
occurred in this source batch. No speed, heat or battery improvement is claimed.

The 2026-09-07 attempt found a wireless ADB service but its connection timed out;
no device was connected. Browser navigation also timed out, so no authenticated
library response, redirect or download was verified in this attempt.

In the grouped ChatGPT round, select a synthetic shared-library attachment from
the production native file sheet and verify actual saved bytes/MIME and route
provenance. Check cancel during transfer, insufficient storage, notification
permission, supported Android storage paths and real VPN/redirect behavior.
Preserve the original draft, conversation and voice session. Test other pending
ChatGPT candidates in the same APK; Google remains after that acceptance gate.

## Personal-library download resolution

The 2026-09-08 extension reuses this candidate and the existing native byte
owner. It does not introduce another file picker, download UI or DOM resolver.

The retained 2026-09-07 public assets were SHA-256 checked again on 2026-09-08:

- `conversation-small-owrec55n6vm0ekcc.js`:
  `7973d518b083f0f3e23905a279ed019378481bdbdd10fc0196afe9fc7b3b7d35`.
  Its preview download callback passes `libraryDownloadId` when the effective
  project is absent, file metadata confirms `is_library_file`, the library ID
  matches, and `is_project` is not true. An omitted project flag is admitted by
  the official condition. Shared and mounted references have separate branches.
- `4813494d-o593jrji51wy4azk.js`:
  `48563cd22f0dafe6c0b89220348fa3add81ff3abb82a62ed9d68a04d569cc375`.
  `uEt`/`OX` resolve effective project ownership through file metadata. `AEt`
  chooses `sharedLibraryFileId ?? libraryDownloadId`; `jX` constructs the same
  `/api/library/files/{id}/download` route already used by this module.

Previously, an ordinary file ID linked to a personal library ID still used
the ordinary JSON download authorization after that metadata check. The native
owner now selects the evidenced binary route instead. The selected file,
library ID and source conversation are snapshotted before the request; mutable
history cannot retarget them. Unconfirmed identity or conflicting metadata
stops the operation before the binary GET. There is no guessed alternative URL.

The metadata read stays bounded to six seconds. Once ownership is confirmed,
the existing 120-second byte-transfer deadline and native save acknowledgement
apply. A transfer may finish after its selection handle expires, but the stale
handle cannot start a second transfer. Account/document/navigation changes and
request-bound cancellation retain their existing checks. A lost final save
acknowledgement is unconfirmed, not permission to replay an ordinary download.

Project-owned files, image pointers and imported connector copies keep their
previous resolver contracts. The extension does not claim those source shapes
are interchangeable with ordinary personal-library attachments, or admit raw
`library_download_id` fields from native commands/history.

Five targeted test cases were added; four failed against the unchanged download
owner, while the rejection case already passed. The new route is covered from
both normal and project conversations, including immutable selection, omitted
project flags, wrong metadata, cancellation, identity changes, storage failure,
lost acknowledgements and exact synthetic saved bytes. The compatibility run
passed 115 cases. The final integration run passed 159 cases with no failures
or skips and additionally covers private history,
conversation-file indexing, shared references and production asset-bundle parsing.
All HTTP and native byte acknowledgements in these tests are simulated.

Status: `code_status=implemented`, `verification_status=offline_verified`,
`device_status=deferred`. This batch does not rebuild or republish the APK.
Next grouped acceptance must verify a real authenticated metadata response,
library redirect/CORS behavior and the resulting saved file through the
production native file sheet. No latency, temperature or battery improvement
has been measured. Citation-only and mounted-library mappings remain separate
research gaps, not completed capabilities.
