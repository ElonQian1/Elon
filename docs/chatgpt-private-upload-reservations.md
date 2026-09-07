# Private upload reservations

Capability: `android_chatgpt_private_upload_reservations_v1`.
Status: production upload integration implemented and offline verified on
2026-09-07; grouped APK build and device acceptance pending. Not `completed`.
The native camera/photo/file actions now start a bounded reservation preparation
when opening the system picker, including new and confirmed existing temporary
chats. Matching selected files reuse it on send; the
existing post-selection byte/image preparation overlap remains available.

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

### Temporary persistence

The same first asset's composer resolves temporary mode as `Br=true`,
`Vr=false`, `Hr=false` and `Ur=undefined` (byte offset 2828582 in the pinned
asset). Its `KJ` call passes those values as temporary, store-in-library and
persistence arguments (2849430). `KJ` preserves explicit false via `eVe`, the
second asset's `jRr` export `V5`; both picker and drag prewarm default only the
reservation persistence with `h ?? 'required'` (1366515/1367155).
`VGt` applies the same default when consuming a selection (468905), and `BGt`
copies that selection value into the final claim. `TGt` still carries
`store_in_library=false` and `is_temporary_chat=true` into processing metadata.
The `required` slot value is therefore not permission to store a temporary
attachment in the personal library. Legacy create/process still omit the field.

## Production ownership

- `chatgpt_web_private_attachment_reservation.js` owns the version-3 ephemeral
  slot, experiment check, exact allocation and final claim shape. It does not
  own native UI, file reading, WebView identity, audio, or message sending.
- `chatgpt_web_private_attachment_selection.js` owns one picker selection. It
  uses already available page identity and a current composer, with a 5-second
  context deadline and a 120-second lifetime. Unknown or project scope does not
  authorize allocation. Temporary scope must retain explicit non-library intent;
  only the reservation protocol normalizes its omitted persistence field.
  The picker never waits for this work.
- `ChatGptWebAttachmentPickerPreparation` binds the selection ID to its page
  generation, document token, route and the exact prepared native file object,
  length and modification time. Empty/multiple selections, expired files,
  launch failure, provider deactivation and page replacement cancel it. No
  file path, bytes, credentials or signed destination enter the selection command.
- On send, the same current binding and transport transfer once. Existing
  conversation membership is freshly confirmed, since moving a chat into a
  project need not change its URL. Changed scope cannot claim an ordinary slot.
  The transport takes a ready compatible slot exactly once instead of creating
  a file. A pending allocation never adds waiting or starts another allocation.
- Without a matching picker slot, the existing sender still starts prefetch
  before byte reads and image preparation. Unsupported or changed file types
  select the established private create route; selection is not an upload.
- A ready slot replaces only file creation and the final processing endpoint.
  Bytes still use the existing signed PUT or same-origin Estuary module.
  Processing must finish for the exact file ID before the existing official
  ready-file association can issue `private_attachment_associated`. A confirmed
  [library reuse result](chatgpt-private-attachment-library-reuse.md) instead
  abandons the slot and byte stage without claiming it, then associates the
  validated existing file once.
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

Native module versions: composer 14, sender 13, selection 2, transport 11,
reservation 3; page adapter 278. The loader registers reservation before
transport and selection before sender. No duplicate picker, sender, polling
loop or system fallback was introduced. The production composer passes a typed
preparation port to its existing attachment actions; work/Google defaults stay
unchanged and no Activity-level event bus was added.

Host-pause bridge cleanup cancels active byte uploads but preserves the bounded,
file-independent selection. Full cancellation still cancels both; provider/page
changes and gateway destruction retain full cancellation. The sender survives
same-version reinjection. Retiring an older native byte lease no longer cancels
a newly opened selection. WebView pause behavior itself is unchanged;
[Android documents](https://developer.android.com/reference/android/webkit/WebView#onPause())
that `onPause()` does not pause JavaScript. No persistent execution lease or
background polling was added for picker prewarm.

## Verification and remaining work

190 focused Node tests passed with no failures or skips across selection, reservation,
production integration, byte routes, transport, composer, image, project,
selected-thread, read-only project, document and native-byte-source suites.
The reservation contract suite has 15 tests; integration has 21, and the
selection lifecycle suite has 6. Integration
uses the production modules and bounded request helper with synthetic Fetch
responses, including PDF/image claims, Estuary byte forwarding, no-wait legacy
selection, incomplete/wrong-file processing, cancellation and current-owner
changes, picker-open ordering, suspension/reinjection, replacement selection,
fresh membership and real production wiring. Temporary coverage includes
new/existing text/PDF/image picker claims, preparation without a picker lease,
zero-wait legacy selection, changed server scope and contradictory library
processing results. The new protocol and picker tests failed on the prior
implementation, then passed after integration. Existing attachment behavior
remains covered. Source-size checks pass. Six new Kotlin owner tests cover
exact-file selection, edits/replacements, cancellation/multiple/expiry,
late callbacks, document changes and background pause; they have not run yet.

No Android build, installation or live reservation/claim ran in this batch.
The grouped device round must retain actual request/association provenance and
verify one fixture through the production attachment UI and a single response.
An account outside the current official experiment is an unobserved reservation
case, not a reason to force the experiment or claim a successful private route.
Actual latency, heat and energy improvement remains unmeasured.

Direct-library hash/reuse and current ordinary composer persistence intent are
now [integrated in source](chatgpt-private-attachment-library-reuse.md), with
212 focused attachment checks passed. Remaining categories and grouped device
acceptance are unfinished. Temporary reservations are implemented, not live-verified;
disabled, pending or incompatible reservations retain the established temporary
create/process path unchanged. Projects keep their established scoped private
upload path and are not converted to ordinary reservations. A `/c/` route
can still belong to a project, so confirmed membership takes precedence.
