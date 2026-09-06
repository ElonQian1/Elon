# Private upload reservations

Capability: `android_chatgpt_private_upload_reservations_v1`.
Status: production upload integration implemented and offline verified on
2026-09-07; grouped APK build and device acceptance pending. Not `completed`.
The current native trigger overlaps reservation creation with file-byte reading
and image preparation. It does not yet prewarm while the system picker is open.

## Confirmed source contract

The public asset fingerprints are recorded in
[the byte-route contract](chatgpt-private-attachment-byte-transports.md).
In `conversation-small-hiw4wce20lu6te81.js`, `Zqt` allocates through
`POST /backend-api/files/upload_reservations`; `Jqt` validates eligible responses
and both expiry strings. `iJt` chooses a ready slot with at least 60 seconds left
on both clocks. `rJt` does not wait for file-picker selections; its separate
drag/drop allowance is not used by the APK file-byte handoff.

The selection experiment is `3119290944`, read via `getExperiment`, not
`getFeatureGate` or `getDynamicConfig`. The shared module's `r6` export is `is`,
which calls `cs().getExperiment`; `t6` provides that page's client. Only the
inspected, already-loaded module
`https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js` is imported. Its
SHA-256 is `89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0`.
Unknown/loading/unrecognized results, warnings, an unassigned group or disabled
`enable` value do not authorize allocation. No experiment override is added.

In `8b34dbc2-kjj15hg4y6iyx13p.js`, `VGt` turns a selected reservation into an
upload entry whose file ID is the reservation ID. After bytes, `BGt` calls
`POST /backend-api/files/upload_reservations/{reservation_id}/claim_and_finish`
and consumes the processing stream through `RGt`. The claim carries filename,
size, use case, persistence intent, MIME, optional image geometry, entry surface
and processing metadata. The PDF model header accompanies the claim. The
`ace_upload` to `my_files` conversion for `.hwp`/`.hwpx` filenames also requests
retrieval indexing; this mapping does not expand the native MIME allowlist.

These are public source observations, not a new successful live claim capture.

## Production ownership

- `chatgpt_web_private_attachment_reservation.js` owns the version-1 ephemeral
  slot, experiment check, exact allocation and final claim shape. It does not
  own native UI, file reading, WebView identity, audio, or message sending.
- The existing sender resolves the current conversation scope first, then
  starts transport prefetch before native byte reads and image preparation.
  It does not await the prefetch. The existing upload transport takes a slot
  exactly once when it would otherwise create a file.
- A ready slot replaces only file creation and the final processing endpoint.
  Bytes still use the existing signed PUT or same-origin Estuary module.
  Processing must finish for the exact file ID before the existing official
  ready-file association can issue `private_attachment_associated`.
- If a slot is pending, ineligible, expired or malformed, the sender proceeds
  with its established private create/upload/process path without extra waiting.
  Taking a pending slot cancels it; a late response cannot replace the chosen
  transaction. A discarded allocation has sent no user file bytes.
- Allocation has a 3-second request limit and a 5-second overall prewarm limit,
  including runtime/identity acquisition. Only one slot exists per upload owner.
  Cancellation or a changed account, document, route, model or composer owner
  prevents further dispatch. Slots and signed destinations are not persisted.
- Both expiry clocks must retain 60 seconds at consumption. Store-in-library
  files larger than 2 MiB retain the official legacy multipart selection rule.
  Unknown destinations reuse the existing byte-route rejection rules.
- No automatic alternate upload or claim is attempted after byte dispatch or
  failed final processing. Private credentials go only to the official origin;
  signed blob destinations receive no account headers or page credentials.

Native module versions: composer 10, sender 9, transport 8, reservation 1;
page adapter 274. The production asset loader registers the reservation before
the transport. No duplicate picker, sender, polling loop or system fallback was
introduced.

## Verification and remaining work

170 focused Node tests passed with no failures or skips across reservation,
production integration, byte routes, transport, composer, image, project,
selected-thread, read-only project, document and native-byte-source suites.
The new contract suite has 13 tests; the new integration suite has 9. Integration
uses the production modules and bounded request helper with synthetic Fetch
responses, including PDF/image claims, Estuary byte forwarding, no-wait legacy
selection, incomplete/wrong-file processing, cancellation and current-owner
changes. Existing attachment behavior remains covered. Source-size checks pass.

No Android build, installation or live reservation/claim ran in this batch.
The grouped device round must retain actual request/association provenance and
verify one fixture through the production attachment UI and a single response.
An account outside the current official experiment is an unobserved reservation
case, not a reason to force the experiment or claim a successful private route.
Actual latency, heat and energy improvement remains unmeasured.

Still unfinished: prewarm from opening the native system picker, the temporary
reservation persistence contract, direct-library hash/reuse, remaining file
categories and grouped acceptance. Temporary and project attachments continue
using their established scoped private upload paths; they are not silently
converted to ordinary reservations. A conversation at an ordinary `/c/` route
can still belong to a project, so confirmed membership takes precedence.
