# Remaining private-native batch

Current implementation audit: 2026-09-07. This is a work list, not a declaration
that every private protocol has been reproduced. Reuse completed capabilities in
[the capability matrix](web-ai-private-transport-capability-matrix.md).

## Workflow

Implement coherent modules with targeted checks and separate commits. Do not
publish an APK for every small correction. Use one grouped install/acceptance
round after the candidate batch is ready. Existing proven native audio,
subtitles, dictation, read-aloud, directory cache, and mutations are not repeated
research. System alternatives remain explicit choices, not silent replacements
for website functionality. Persistent WebView identity remains intentional.

Latest user priority: finish the remaining ChatGPT private capabilities first.
Google work is last; do not start Google protocol research, implementation or
acceptance while these ChatGPT gaps remain. Completed capabilities stay reused.

## Current batch

| Work | Code | Verification | Delivery |
|---|---|---|---|
| Private history to native wire format, citations, file/image descriptors | Implemented | JS and shared Android fixture passed | Published/installed 1540; device UI acceptance pending |
| Content-only refresh preserves current composer/voice state | Implemented | Targeted Android tests passed | Published/installed 1540; device UI acceptance pending |
| Bounded/coalesced image requests and no false empty-library success | Implemented | Targeted JS passed | Published/installed 1540; device UI acceptance pending |
| Private conversation attachment index, cache and native file sheet | Implemented | Shared JS/Android contract and targeted production tests passed | Published/installed 1540; device UI acceptance pending |
| End-to-end private read deadlines, body limits and late project response isolation | Implemented | Lifecycle and existing JS consumer suites passed | Published/installed 1540; device acceptance pending |
| Bounded request-shape capture through native MCP, reusing the page observer | Implemented diagnostic only | Node/Android checks passed; actual reservation JSON and conversation SSE capture observed | Published/installed 1540; telemetry-budget follow-up is source-only |
| Reservation responses cannot prematurely release attachment sends | Implemented regression correction | Node red-to-green, 12 Android tracker tests passed; 1541 production reply read the actual file content | Published/installed 1541; no early-send symptom in the single fixture test |
| Private file create/blob upload/process transaction | Implemented transport and native byte/store integration for one plain-text file in an empty ordinary new-chat or existing-chat composer, with confirmed conversation scope | One earlier 78-byte private upload processed; grouped checks passed; 1541 production file-content acceptance passed, but private-association provenance was not retained | Published/installed 1541; integrated private-route confirmation pending; [scope and contract](chatgpt-private-attachment-upload.md) |
| Private static-image attachment upload | Native normalized JPEG/PNG/WebP handoff, bounded image preparation, multimodal private upload and ready-store dimensions implemented | Included in 96 Node cases and grouped Release build; actual image-reading acceptance pending | Published/installed 1541; [image contract and acceptance](chatgpt-private-image-upload.md) |
| Private temporary-chat attachments | New/existing temporary scope, non-library processing intent and exact text/image ready-store association implemented | Included in 96 Node cases and grouped Release build; live upload/library checks pending | Published/installed 1541; [scope contract and remaining project work](chatgpt-private-attachment-scopes.md) |
| Private new-project attachments | Fresh permission read, scoped text/non-ingest-image upload and library-file ready-store metadata implemented for new project chats | Included in 96 Node cases and grouped Release build; project/library acceptance pending | Published/installed 1541; read-only/ingest-image extensions are source-only below; [exact scope](chatgpt-private-attachment-scopes.md#project-checkpoint) |
| Private existing-project attachments | Fresh membership plus official selected-branch binding; scoped text/non-ingest-image processing and ready-store origins implemented | Included in 96 Node cases and grouped Release build; actual module access and project upload pending | Published/installed 1541; [branch contract](chatgpt-private-attachment-scopes.md#existing-project-branches) |
| Private PDF attachments | Existing native byte lease, model-bound create request and ordinary/temporary/writable-project association extended to PDF | 96 Node cases and 33 grouped Android tests passed, including PDF MIME/size policy; actual PDF acceptance pending | Published/installed 1541; [protocol and scope](chatgpt-private-pdf-upload.md) |
| Attachment cancellation without UI-thread I/O | Immediate byte-lease revocation, off-thread file cleanup and stale-read exclusion implemented | All 6 native reader tests passed, including blocked-read/EOF cancellation; grouped Android 33/33 passed | Published/installed 1541; live slow-read cancellation not exercised |
| Private conversation file download authorization and native transfer | Implemented scoped private GET, expiring selections and production Download action | Official current source contract, targeted JS, Release compilation and Android checks passed; device transfer pending | Published/installed 1541; [scope and contract](chatgpt-private-file-download.md) |
| Project/library-linked conversation attachment downloads | Existing native Download action now resolves confirmed project scope and library file metadata before private authorization | 48 focused Node cases passed with synthetic HTTP; actual saved bytes and live scope acceptance pending | Published/installed 1541; [scope extension](chatgpt-private-file-download.md#project-and-library-extension) |
| Private single-conversation deletion | Both evidenced DELETE/PATCH branches now select from the current official recognized flag; existing native confirmation, voice/send exclusion and exact cache invalidation retained | 18 deletion JS cases pass; current runtime binding and live deletion pending | Legacy branch installed in 1541; flagged version 3 is source-only for grouped APK; [scope and remaining work](chatgpt-private-conversation-delete.md) |
| Existing file index and upload receipts exposed through production MCP | Current-document/current-route file descriptors and expiring opaque selections; upload receipt survives unrelated send/skin receipts | Release compilation and 43 targeted Android tests pass; no new network/DOM path | Source-only for grouped APK; [contract](chatgpt-private-conversation-files.md#production-mcp-acceptance-access) |
| Private generated-image library pagination | Existing native gallery now reads the official cursor catalog using the active identity WebView; scoped previews reuse the bounded image cache, with previous/next and cancellation | 18 focused Node cases, 79 asset syntax checks, Release source compilation and 33 Android tests pass | Source-only for grouped APK; [contract](chatgpt-private-image-gallery.md); live generated catalog and previews pending |
| Private images in ingest projects | Existing upload transaction now resolves the current official retrieval gate, reusing the selected-branch runtime and exact project association | 101 targeted Node cases pass, including new/existing project flags, cancellation and production asset-bundle parse | Source-only for grouped APK; [contract](chatgpt-private-attachment-scopes.md#ingest-project-images); actual account gate and indexed-image acceptance pending |
| Private attachments in read-only project chats | Production upload now separates chat-only attachment permission from project file writes, preserving native and official ready-store association | 110 targeted Node cases pass, including new/existing text/image/PDF, exact request scope, stale/cancel guards and asset-bundle parse | Source-only for grouped APK; [contract](chatgpt-private-attachment-scopes.md#read-only-project-chats); actual membership, reply and file placement pending |
| Private common-document attachments | Existing native File action and byte lease now admit Word/Excel/PPT, ODT/RTF, CSV/TSV, Markdown, JSON/XML/HTML alongside text/PDF, with project indexing and chat-only permission scope | 115 targeted Node cases pass, including native/source MIME parity, production module pipeline, byte integrity and scope combinations; new Kotlin tests not yet executed | Source-only for grouped APK; [contract](chatgpt-private-document-upload.md); grouped compilation and actual document-reading acceptance pending |

Root cause, exact modules, and check results are in
[the history contract](chatgpt-private-history-native-contract.md).
The attachment-index scope and acceptance are in
[the file index contract](chatgpt-private-conversation-files.md).
Request ownership and the confirmed baseline failures are in
[the request lifetime contract](chatgpt-private-request-lifetime.md).
The opt-in research command and its limits are in
[the protocol evidence contract](chatgpt-private-protocol-evidence.md). It is not
a replacement for any missing business protocol below.
Five legacy source-location assertions also fail on the unchanged baseline;
their exact scope is recorded there. They are not a full-suite pass or a reason
to repeat already-verified private transports.

## Grouped release

### Current 1541 acceptance

On 2026-09-06 after the handset returned, the grouped Release production and
unit-test compilation passed all **33 tests across seven attachment suites**,
with zero failures, errors or skipped cases. The latest focused Node run passed
96 cases. These are targeted checks, not a full regression or thermal A/B.

`publish-apk.ps1` published `v1.1.1541` (code `1541`) from `ac2f1662f` and verified
the remote APK size and SHA-256:
`15e20f7cda24e0bfc2a9b7c67fb2884141c2d159cddec95463328488f4a0ef4a`.
The whitelisted postflight installed it on Xiaomi 14 Pro using replacement
installation and read back build 1541. Cookies and application data were kept.
Both ChatGPT and Google returned HTTP 200 in the APK network check before the
grouped acceptance. No accelerator configuration or core was changed.

The production social-AI chat successfully staged and removed the fixed text
fixture, then sent it **once**. The native attachment state reached `completed`,
pending count became zero, and the assistant reply contained both the unique
request marker and the fixture's first line, which was not supplied in the
prompt. The initial assertion stopped on a PowerShell closure failing to resolve
its named helper, not an APK upload failure. Capturing the helper scriptblock
fixed that boundary; the contract test now executes the real predicate across
a module boundary. Resuming the persisted `reply_requested` checkpoint verified
the existing reply without dispatching another message. The fixture was removed,
the production acceptance case registered, and the phone returned to its original
conversation-home surface. No microphone was used.

This proves the production file-delivery workflow, **not** which upload route
ran: its private-association receipt was not retained, and the latest command
had already advanced to send/skin state. Do not count it as integrated private
upload, image, PDF, project or saved-download acceptance. Collect existing
semantic receipts during the next scoped check; do not repeat protocol research
or rebuild the unchanged APK merely to recover that missing evidence.

### Earlier 1540 checkpoint

On 2026-09-06, `publish-apk.ps1` built and published `v1.1.1540` (code `1540`)
from `ccc76ed37e31364f02c03af333a13a63b30c4bdf`. Remote version, size and SHA-256
were verified. APK SHA-256:
`ef29913013d10a170e16a1ce7d8a2648377495edabeb3f0c6fb62c26eb67755c`.
The standard whitelisted-device postflight used `adb install -r` and read back
build `1540` on Xiaomi 14 Pro. Cookies and application data were preserved.

Installation is not production UI or protocol acceptance. MCP health initially
responded after the update, but later health calls timed out and a plain ADB
process query returned `error: closed`. Both existing command helpers experienced
failures at different times, so there is no confirmed helper-specific defect.
No protocol-capture lease, synthetic upload, new message, or microphone test was
started in that initial installation round. Browser navigation also timed out;
it supplied no protocol evidence.

The resumed round reconnected the same handset. Its accelerator `1.0.139 (140)`
crashed on a missing JNI restore method; the accelerator owner fixed and
installed `1.0.140 (141)` without this task changing proxy code or settings.
After network recovery, one new synthetic attachment attempt through production
`send_input` completed and produced a native streaming acknowledgement. The
capture observed HTTP 200 reservation JSON and official conversation SSE. It
did not establish the complete upload/finalize protocol or independent private
dispatch. The probe was cleared and the UI restored to conversation home with
an empty draft and no pending attachment; details and limits are in the
[recovered-network capture](chatgpt-private-protocol-evidence.md#recovered-network-capture).

The candidates implemented the narrow prepare/upload/finalize and
composer-association contract and are now included in 1541 above. Continue the
remaining route-specific acceptance with bounded production MCP commands; do not
implement the same transport again.
The [reservation regression](chatgpt-private-protocol-evidence.md#reservation-completion-regression)
invalidates generic HTTP completion as upload proof; include that correction in
the grouped candidate before accepting attachment delivery. First confirm a healthy transport and preserve the current
draft, conversation and voice state. Do not rebuild this unchanged candidate,
add another probe framework, guess an endpoint, or repeatedly restart the app
because the debugging connection is unavailable. The Goal is not complete.

## Offline lifecycle checkpoint

The 2026-09-06 evening round reconnected Xiaomi over wireless ADB. The user then
took the handset away, so installation, fixture upload and UI acceptance were
deferred; successful ADB connection is not functional acceptance. The grouped
JavaScript runner passed 76 cases without a phone. The first Android check was
terminated by the command wrapper's 180-second no-output threshold during Kotlin
compilation, not a reported source compilation error; its result is not a pass.
The subsequent run compiled Release production and unit-test sources, then
identified one pre-existing attachment-panel test that still asserted obsolete
button labels. That test now checks stable selectors bound to the actual camera,
photo and file handlers. The final grouped run passed all 32 targeted Android
tests (tracker 12, download policy 5, native byte reader 6, native MIME/image
policy 3, fixture/native actions 5, production panel contract 1). This is not a
full-suite, rendered UI, upload protocol or thermal acceptance claim. No APK was
packaged, published or installed in this offline checkpoint.

Review found that the native byte gateway synchronously called a synchronized
reader close from the main thread. A slow read could therefore delay cancellation
or conversation switching. Revocation now invalidates the lease immediately;
file close drains on the existing I/O executor, including disposal. Blocked reads
reject revocation, and the gateway rejects late bytes from an expired lease; a
revoked reader cannot reopen the file. The upload endpoints, identity
ownership, voice modules and provider fallback policy are unchanged. Include this
fix in the next grouped APK and verify it through the production chat surface.

The next offline batch adds existing-project attachment branch ownership using
the inspected, already-loaded official module rather than the most recent server
node. It passed 86 focused JavaScript cases, including the full asset-bundle
parse. It did not rerun Android compilation, publish another APK or operate the
absent handset. The preceding 32 Android tests do not establish live acceptance
of this later JavaScript extension. Reuse this implementation for grouped testing;
read-only project dispatch and ingest-image flags were gaps at that checkpoint
and are now implemented in the source-only candidates above; multipart remains.
The
subsequent [PDF extension](chatgpt-private-pdf-upload.md) is now implemented and
included in the 96-case focused Node run. Its Kotlin checks subsequently passed
in the 1541 grouped build above; actual runtime model binding remains pending.
The offline implementation itself did not operate the absent phone.

## Protocol gaps

| Area | Existing usable path | Actual remaining private work |
|---|---|---|
| Text send/regenerate | Native send ledger and official transaction; streaming observer | Fresh proof-bound private dispatch is not verified. Do not replay captured proof headers or declare official fallback a private POST success. |
| Model/effort/tools/temporary mode | Native presets/cache and official controls; [search/image private live-state bridge](chatgpt-private-composer-tools.md) and [current-version model/effort preset bridge](chatgpt-private-model-state.md) implemented in source | Accept both pinned runtime bridges in grouped device testing; finish advanced/version/work-model, service-tier and temporary-mode mutation. Cached labels or local readback do not prove asynchronous server-preference persistence, and tool selection is not an independent generation POST. |
| Attachment upload | Earlier small-text private upload verified; production text-file delivery passed on 1541; native byte handoff and ordinary/temporary/writable-project image/PDF transactions packaged; ingest-project images, read-only project chat uploads and [common documents](chatgpt-private-document-upload.md) now implemented in source | Confirm integrated private-route provenance, image/PDF/Office reading, official model/branch/flag-runtime access, read-only file placement and temporary/project library behavior; finish remaining MIME categories and multipart/reservation/direct-library variants. Production file delivery is not a private-route pass. See [upload contract](chatgpt-private-attachment-upload.md), [PDF extension](chatgpt-private-pdf-upload.md) and [scope contract](chatgpt-private-attachment-scopes.md). |
| Images | Native gallery/previews/cache; private generated-catalog pagination and scoped previews now implemented; official creation preserved | Accept the [private gallery candidate](chatgpt-private-image-gallery.md) on the device, cover remaining pointer scopes, and implement the confirmed generation transaction. Do not repeat catalog discovery or count source tests as live API success. |
| Share/delete/conversation files | Native pin/rename/archive/move; private file index plus ordinary/project/library-linked and [simple image-pointer download candidates](chatgpt-private-image-download.md); guarded flag-selected private delete and current-chat reset | Verify saved-file downloads, live flag binding and current/noncurrent deletion. Complete shared-library/connector/parameterized-image download scope and official sharing. Do not substitute system sharing for official features. |
| Google direct send (last) | Native cache and private response observer; official submit | Deferred until remaining ChatGPT private work is ready. Then reproduce the submit contract and transaction ownership; observed reply endpoints do not imply a working private sender. |

An unknown protocol remains a documented code gap. No guessed endpoint, fake
success, or automatic write replay should be added merely to make this table
look complete. Existing UI stays usable while each replacement is implemented.

## Sharing protocol checkpoint

The official `8b34dbc2-kjj15hg4y6iyx13p.js` asset read on 2026-09-06 has SHA-256
`9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`.
Its `aAn`/`bAn` message-slice flow posts `post_text` and
`attachments_to_create: [{kind: "message_slice", conversation_id, message_ids}]`
to `/share/post` or flag `1308952433`'s `/share/post/link` branch. This is not proof
that the full-conversation Share action uses that writer. Its `jAn` instead awaits
an externally supplied `shareCreatePromise` with `shareLinkUrl`, `shareLinkId` and
`currentNodeId`. `Kkn` derives preview IDs from the selected conversation branch;
it does not create the link. The remaining research entry is the full-conversation
creation caller/lazy module and exact response contract. No link was published,
no guessed endpoint was implemented, and system `navigator.share` is only the
subsequent distribution step after an official link exists.

Further public-source inspection confirmed a separate project-member sharing path:
`conversation-small-hiw4wce20lu6te81.js`'s `Gkt` gates project/temporary/health
contexts before choosing a share flow. The lazy `ShareProjectChatModal` in
`/cdn/assets/0ec7d136-iapesa5f61fnsuwq.js` (SHA-256
`c720e9928fbca61b5e9c8a4b19eef020f1e68cc554d8598e281880552958d72c`)
copies a link accessible only to existing project members and explains that future
messages remain visible to those members. It does not create a public share by
POST in that module. Do not substitute this membership-scoped link or the
message-slice writer for the missing whole-conversation public-link creator.

The next offline trace located `Gkt`'s ordinary-share entry in `Sm.openSharingModal`:
it writes `sharingModalThreadId` and overrides into the official UI store, rather
than issuing the creation POST itself. The whole-conversation creator remains in
the unsampled consumer of that store. The existing PC browser bridge failed its
bounded retry, so no additional live caller or request was observed and no share
link was created. Continue from that consumer when browser access is available;
do not repeat the already-inspected message-slice or project-member paths.

On 2026-09-07 the in-app browser opened the guest homepage, whose loaded
`unauth-mweb` assets did not expose that authenticated modal consumer. The
existing Chrome connection timed out. No new share writer was confirmed and
no share link was created; image download implementation progressed separately
using the already verified public asset contract above.
