# Private attachment library reuse

Capability: `android_chatgpt_private_attachment_library_reuse_v1`.
Status: production source integration implemented and offline verified on
2026-09-07. Grouped Android build and live acceptance pending; not `completed`.
This is page-local private HTTP using existing WebView identity, not independent
Android HTTP, a new file picker, or a second message sender.

## Current source evidence

The pinned composer/shared assets and their SHA-256 fingerprints are in
[the byte-route contract](chatgpt-private-attachment-byte-transports.md).
The shared runtime fingerprint is in
[the reservation contract](chatgpt-private-upload-reservations.md).

In the composer asset, `j$n` receives `isLibraryEnabled` and `entrySurface`.
Its `Br/Vr/Hr/Ur` values pass temporary mode, library storage and persistence to
`KJ`; `eVe` (the second asset's `jRr` export `V5`) preserves explicit choices.
An enabled ordinary composer uses `store_in_library=true` and
`library_persistence_mode=opportunistic`. Temporary chat passes false and omits
legacy persistence. Reservation-only defaults do not change that privacy intent.

`gB.uploadFile` enables the reuse callback only for a library-enabled,
authenticated, non-temporary, non-project chat composer without a custom GPT
scope. `Ywe` (second asset `uqt`, export `K$t`) accepts current gate `1342446482`,
or gate `2354748696` plus experiment `2711463943` with `enable=true`.
`EGt` hashes file bytes with SHA-256. `DGt` sends
`POST /backend-api/files/library/reuse` with `file_size_bytes` and
`sha256_digest`, then reads `reusable_library_file`.

`OGt` runs this lookup alongside the byte upload and cancels lookup after bytes
are accepted. `kGt` awaits the losing upload: a genuine upload success wins;
only reuse-triggered cancellation may select the existing library entry.
`VGt` then skips final processing for that abandoned upload and invokes
`handleLibraryFileReused`. The latter sets `source=library`, `autoReused=true`,
the returned file/library IDs and file-spec name while retaining the selected
local File. A response alone is not permission to create two ready entries.
These are public JavaScript observations, not a live reuse capture.

## Production implementation

- Composer version 14 reads the current React branch's existing library option
  together with its model slug. Identity, document, route, store, current branch
  and this option bind an upload. Unknown options do not enable storage. The
  option is never rewritten just to make reuse possible; no new settings or
  synchronous settings request was added.
- `chatgpt_web_private_attachment_library.js` version 1 owns reuse eligibility,
  current recognized gate/experiment lookup, hashing, bounded private lookup
  and race ownership. It imports only the pinned already-loaded/preloaded
  official runtime, disables exposure logging and never overrides flags.
- WebCrypto computes the same SHA-256 over the exact selected/prepared file.
  The existing 8 MiB cap bounds its additional buffer; this is not the website's
  incremental stream-hash implementation. Runtime, hashing and lookup share a
  1.5-second opportunity window. Upload begins concurrently and never waits for
  this probe to miss, time out, or load a runtime. There is no periodic polling.
- Lookup retains same-origin identity and allows at most 16 KiB of JSON.
  Returned IDs, filename and optional MIME must validate. Unknown, failed or
  malformed lookup leaves the existing upload running, rather than restarting
  or replaying it. A late hash/response cannot dispatch or associate after its
  owner becomes stale.
- Transport version 11 isolates byte-stage cancellation from the parent upload
  owner. A hit aborts and awaits bytes before selecting reuse. An acknowledged
  upload remains authoritative; a genuine byte error is not masked by reuse.
  Whole-file, Estuary and multipart paths use the same existing byte module.
  Multipart cancellation forbids block-list commit; reused files skip the
  abandoned upload's processing or reservation claim.
- Composer association validates explicit library scope, the exact owner and
  original selected-file descriptor, then reads back one official ready-store
  object. The official spec uses the existing server filename/IDs; the native
  pending-file row retains the user's selected filename and Remove action.
  No automatic message, uploaded-file deletion or duplicate association occurs.
- Projects, temporary chats, disabled library settings and unsupported contexts
  retain their existing upload contracts. Reservation version 3 honors ordinary
  opportunistic intent, skips direct-library files above 2 MiB and still applies
  the established temporary reservation-only default. Sender 13 and adapter 278
  load the library module before transport and retain single-owner reinjection.

The implementation reduces duplicate transfer only when an eligible lookup
wins. It does not promise zero bytes uploaded or zero orphaned allocation; the
website also races these operations. Cancellation does not prove that a server
never received bytes. The client does not claim or associate the abandoned
allocation and does not issue speculative deletion.

## Verification

212 focused Node cases passed with no failures, cancellations or skips across
library policy/reuse/production integration and the existing reservation,
selection, bytes, transport, composer, thread, image, project, read-only,
document and native-source suites. The new reuse and integration suites contain
15 tests. They use synthetic responses and the production bounded Fetch helper,
transport, sender and ready-store association, not a replacement upload engine.

Coverage includes exact SHA/request shape, current recognized flags, unchanged
retention choices, slow-probe bypass, miss/error retention of the original
upload, success-versus-cancellation ownership, account/document/option changes,
reservation abandonment, multipart/Estuary cancellation, PDF model headers,
image dimensions, one ready object, removal and production bundle registration.

No new APK, live library request, device screenshot, speed measurement, or
thermal/energy comparison was performed in this source batch. During grouped
ChatGPT acceptance, use a synthetic fixture and retain actual selected route
plus native association evidence. Where the official account enables reuse,
verify one reused file is associated and its contents reach one reply; where
disabled or unobserved, record that limitation instead of forcing the gate.
Existing browser login and original conversation/draft state must be preserved.

## Explicit upload-copy choice

The current native source adds a pre-send choice, rather than reproducing the
website's post-reuse toast. The inspected composer asset's `VGt` upload-anyway
callback invokes the same uploader with `checkForReusableLibraryFile: false`
and `takeUploadSlotPrefetchSelection: undefined`. This is source evidence for
bypassing reuse and its earlier slot, not a new HTTP endpoint or a live capture.

In production ChatGPT chat, long-pressing a supported pending attachment opens
the existing native popup style with reuse-first and upload-copy choices. The
default remains reuse-first. `PendingAttachment.chatGptUploadCopy` belongs only
to that exact local selection, not to an account, project or global setting.
The choice is available before upload, with no DOM or network synchronization.
Copy requires one file accepted by the existing private MIME/size/image policy;
an earlier copy choice can still be reset after adding another file. Existing
image preview/edit/remove controls, Google and work-mode paths are retained.

The menu checks active ChatGPT ownership, no streaming/pending upload, the
original conversation and an attached anchor. The mutation uses reference
identity, so a removed, edited, replaced or submitted file cannot be changed by
an old menu. Choosing the current mode does not replace the file or invalidate
its picker preparation unnecessarily.

Adapter 287, sender 15, transport 12 and library 2 carry the choice through the
existing native byte lease and upload owner. Explicit copy cancels picker
preparation, does not take or create a reservation, skips library import/hash/
lookup, and uses the existing create/byte/process/association transaction.
Transport independently enforces this even with an older library module and
snapshots the option before async authentication. The internal choice is not
added to the official JSON request body; temporary/project/library retention
continues to follow the existing official composer policy.

Explicit copy cannot silently fall back to a chooser/DOM path that could reuse
the file. Unavailable private upload fails the reserved send without submitting
its text, while default selection retains existing compatibility behavior.
After a private allocation, cancellation or context loss cannot associate or
replay. Already allocated server slots are not speculatively deleted. This
guarantees the client uses selected bytes, not that the server stores duplicate
physical bytes. Replacing an already-associated file after sending is not part
of this pre-send interaction.

Verification: four new cases first failed against the previous sender/library.
The final focused run passed all 247 Node cases, including actual production
sender/selection/reservation/bytes integration, stale/cancel guards, native
wiring contracts and the asset-bundle parse. Fifteen Kotlin/JUnit cases passed
for selection identity and send ownership. Current menu, preview, model and
send-owner sources also compiled against the Android SDK and cached untouched
app dependencies; this is not a full Android build. Gateway and large-entry
consumer compilation, popup rendering and real request/byte/reply acceptance
remain for the grouped APK. No phone, browser session, microphone, release or
thermal benchmark was used in this source-only extension.

Remaining work includes unsupported file categories and grouped live
gate/association/copy-choice acceptance. Do not rediscover the endpoint or
reimplement these modules while those checks wait.
