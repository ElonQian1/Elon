# Remaining private-native batch

Current implementation audit: 2026-09-10. This is a work list, not a declaration
that every private protocol has been reproduced. Reuse completed capabilities in
[the capability matrix](web-ai-private-transport-capability-matrix.md).
Operation admission and remaining composer dependencies: [readiness policy](chatgpt-operation-readiness.md).

## Workflow

Use the [private integration playbook](web-ai-private-integration-playbook.md) for evidence acquisition, known pitfalls, scoped completion and Google reuse; it is not a second capability register.

Implement coherent modules with targeted checks and separate commits. Do not
publish an APK for every small correction. Use one grouped install/acceptance
round after the candidate batch is ready. Existing proven native audio,
subtitles, dictation, read-aloud, directory cache, and mutations are not repeated
research. System alternatives remain explicit choices, not silent replacements
for website functionality. Persistent WebView identity remains intentional.

Latest user priority: finish the remaining ChatGPT private code first, then
perform one grouped Android build/install and production-phone acceptance of
the completed ChatGPT workflows. Fix that round's failures before starting
Google protocol work. Google receives its equivalent implementation and
acceptance only after this ChatGPT gate passes. Completed capabilities stay
reused; source-only or installation status does not satisfy the acceptance gate.
Latest user clarification: finish and accept the remaining private-API functions
before thermal/battery optimization. Heat measurements are deferred and are not
a functional acceptance gate; do not expand performance tests in this phase.

## Current Status Map

This summary supersedes older delivery labels below; it does not broaden a
passed scope. Bounded ordinary/project directory continuation is now completed,
default-enabled and device-verified on normal 1611, adapter 311; see
[owned continuation](reports/chatgpt-directory-owned-continuation-20260909.md).
Installed normal 1612 additionally passed the actual native file-library Download
button and saved-byte checks for TXT/PNG/PDF without a ready composer; see
[rendered download acceptance](reports/chatgpt-library-download-ui-20260909.md).
[Explicit full-page browsing](chatgpt-private-directory-browser.md) is completed
and production-verified on 1614: next/previous, project folder/back and existing
conversation navigation. Large-account overflow remains synthetic coverage,
not a claim that the whole live account was traversed. Project TXT/PNG/PDF
upload, native send and actual file-content replies now passed on 1614; the
post-send membership timing defect and source fix are recorded in
[project-media follow-up](reports/chatgpt-project-media-1614.md).

| User workflow | Current result | Remaining boundary |
|---|---|---|
| Conversation/project cache, native audio/subtitles, private dictation/read-aloud | Previously device-verified, reused | Do not repeat without regression evidence; identity WebView remains intentional |
| Pin, rename, archive, move conversation to a project | Device-verified, reused | Other account/permission variants are not implied |
| Ordinary new-chat text + PNG + PDF attachments | Completed on 1598 | Project/temporary/other-format variants remain separate |
| File library navigation, saved download, single library attachment, rename/soft-delete/upload-copy | Earlier scopes through 1591 reused; native TXT/PNG/PDF Download buttons and saved bytes passed on 1612 | [Mounted catalogue download](chatgpt-private-library-mounted-download.md) (244 checks) and [mounted attachment preparation](chatgpt-private-mounted-library-attachment.md) (196 checks) published in normal 1631; wireless installation timed out, device acceptance pending. Large-transfer/cancel/crash cases, other special sources, folder writes and attachment scopes remain |
| Personal full sharing/list/revoke and current-conversation delete | Scoped personal cases completed on 1547; 1623 additionally passed account-wide personal-list read and native menu/list display (24 links) after correcting a pathless-account request forwarding bug | Large-account cached-page controls and rendered Copy/cross-conversation revoke remain unclaimed; workspace/Canvas/post/bulk and server pagination remain separate; [contract and acceptance](chatgpt-private-shared-links.md) |
| Text submit/stream/stop/follow-up | Official runtime path verified, reused | Not independent Android HTTP POST. Regeneration v6 removes post-dispatch picker lookup, passes 201 checks and is installed on 1630; native retry awaits unlock. The earlier 1629 attempt stopped before retry; [evidence](reports/chatgpt-native-regeneration-20260910.md). Other contexts remain separate |
| Project attachments | Native new-project TXT/PNG/PDF upload, send and actual content reading verified on 1614 | Explicit membership query rejected when background freshness expires; fix passes offline tests, grouped install/idle-query acceptance pending |
| Model/effort/tool combinations | 1625 Advanced/version and 1627 cached High/Extreme selection reused; native new blank chat and temporary on/off passed. 1628 fixed premature tool-read cancellation and passed authenticated native Create Image/Search on/off, private receipts and chip clearing; [evidence](reports/chatgpt-composer-state-20260910.md) | Physical drag, other model/tier/tool and temporary contexts, subsequent send and server preference persistence remain unclaimed |
| Latest cursor image gallery | Installed 1620 passed native gallery entry, new page 4 ready, viewer and return; v8 passes 52 Node + 7 JVM checks | 1628 actual creation returned completed image parts, but the last-row smoke misclassified it; corrected full acceptance remains pending ([evidence](reports/chatgpt-composer-state-20260910.md#generated-image-reply-evidence)). Thumbnail efficiency/thermal work is deferred; [gallery evidence](reports/chatgpt-gallery-thumbnails.md#usb-acceptance-on-1620) |
| Explicit file citations / remaining cloud references | Concrete and grouped/cite-map ChatGPT file-ID citations reuse native index/download. The v16 metadata/project-scope and cancellation correction passes 203 checks and is installed on 1630; production citation download remains pending | [Boundary and source evidence](chatgpt-private-file-citations.md). Other mounted providers, URL-only cloud and PCA citation graphs remain incomplete |
| Google private sender | Deferred by user priority | Start only after the remaining ChatGPT acceptance gate |

Current code correction separates current-document navigation/private directory
access from composer readiness. Sending, attachment submission and new-chat
confirmation keep their existing gates. Current investigation and delivery
evidence are in [the project readiness report](reports/chatgpt-project-media-acceptance-20260909.md).

The [library attachment deadline correction](chatgpt-private-library-attachment-deadline.md)
aligns page-operation/native-receipt/UI budgets and prevents late publication.
202 Node and 8 JVM checks passed; the correction is included in normal
1632/1634/1635. Slow mounted-file device acceptance remains pending.

[Library append](chatgpt-private-library-append.md) reuses the private attachment
owner for consecutive Library files and Library files after a local-upload batch,
with the official count validator, duplicate handling and exact submit cleanup.
[Release 1635](reports/chatgpt-library-append-1635.md) accepts consecutive native
TXT/PDF selection, two ready cards, one runtime send, both file-content markers,
cleanup and original-conversation restoration. Reuse this completed ordinary
scope; reverse-order local uploads, expired visible handles and real mounted
append remain incomplete.

## Recent Accepted Scopes

[Normal Release 1589 evidence](reports/chatgpt-library-mutations-1589.md):
ordinary native library rename/soft-delete, explicit one-text-file upload-copy
with private upload and file-content reply, and rename/search reconciliation
passed. Originals and the blank chat/draft were restored. These scopes are
completed and reused. [Normal 1591 navigation acceptance](reports/chatgpt-library-scopes-1591.md)
subsequently passed native More (21 to 41 rows), folder and back. Reuse these
completed scopes. Normal 1595 new temporary single-text attachment passed private
upload/send, actual file reply, selected/read-only state and unchanged library
fixture count, with restoration. This narrow scope is completed; variants remain pending.

Latest mixed-media acceptance: [normal APK 1598](reports/chatgpt-media-batch-1596.md). Ordinary new-chat mixed text/PNG/PDF private upload, accepted native send, actual three-file reading and restoration are **completed**; reuse this scope. Other format/context variants remain pending.
Ordinary authenticated native stop/follow-up is **completed**: both sends and
stop used the official runtime, all 229 observed partial characters survived,
and four ordered separate rows remained. Original blank chat/drafts were restored.
Reuse this scope without repeating acceptance unless there is new regression.
The older [APK 1576 guest continuity failure](reports/chatgpt-stop-followup-1576.md)
has not been retaken; do not clear login to recreate it or infer its root cause
from the authenticated pass. The unchanged native harness checks a requested
public-answer prefix, separate turns and restoration without mutating the tree.

## Historical Delivery Checkpoints

These paragraphs preserve what was proven at each earlier build. They are not
the current installed version or a reason to repeat subsequently completed work.

Grouped delivery `1542` compiled and passed 1,007 Android tests, was
published and installed, and confirmed one production private text-file upload
with actual file-content reply. `1543` includes the file-index freshness fix;
the production private index returned the same fixture in 716 ms. Its download
returned `download_source_unsupported`; no saved-file acceptance is claimed.
`1544` adds the source-evidenced same-origin content download candidate through
the existing native byte/save owner. Its 118 related Node runner cases and
Release build passed; publication, replacement installation and MCP build
readback passed. After unlock, the ordinary fixture download saved 78 bytes in
1,960 ms and its device SHA-256 matched the original. This narrow saved-bytes
case is completed; project/library variants and rendered-menu acceptance remain
pending. The rejected 1543 URL shape has not been captured.
See [the grouped release evidence](reports/chatgpt-grouped-release-20260907.md).
The source candidates below are bundled through `54a89232e`; older delivery
cells are historical checkpoints, not current source-only blockers. Bundling
does not upgrade any pending protocol or rendered-UI acceptance claim.

Correction delivery APK 1545 includes same-origin generated-image preview
v4, shared-link management v2 and on-demand public-script inventory through
existing MCP diagnostics. Related Node suites, six focused Android tests and the
Release build passed. Wireless replacement installation was verified. The real
private share-list read returned a complete empty result in 1,436 ms; inventory
also worked. Gallery UI and public-share creation/revocation remain pending.
No normal-user polling, account changes or independent proxy changes were added.

Delivery `1.1.1547` with [versioned website runtime bindings](chatgpt-private-runtime-bindings.md)
replace stale module selection across sharing, deletion, model state, text,
regeneration, stop, temporary chat and attachment scope/reuse. The September 7
website changed both filenames and export aliases. New/old-build and consumer
regressions, Release build, publication and replacement installation passed.
Personal share/create/list/revoke and current-conversation deletion passed using
one synthetic fixture through production handlers. The native reply arrived via
the existing `template_unavailable` fallback, with an extra thinking-status bubble;
direct sending is not accepted. [1547 evidence](reports/chatgpt-runtime-release-1547.md)
records that historical checkpoint; subsequent text-send acceptance is above.

Grouped delivery [APK 1552](reports/chatgpt-runtime-release-1552.md)
contains the resolver/backoff, [new-chat confirmation](chatgpt-private-new-conversation.md),
mounted-file changes below and the resident-composer-host correction. Release
compilation, publication and replacement installation passed. One native text
send received its exact reply through DOM fallback; runtime submission remains
`submission_not_ready`, and native generating state did not settle with the
completed stream. Neither direct-send nor guest-confirmation acceptance passed.
Reuse these modules and fix the recorded readiness/state failures before Google.

The subsequent [mounted-file download candidate](chatgpt-private-mounted-file-download.md)
adds concrete Drive/Box/Dropbox conversation references and Drive Docs/Sheets/Slides
exports to the existing native download owner. Adapter 297 binds server-resolved
name/MIME to a consumed native lease for both byte storage and signed downloads,
preserving exported suffixes and bounded Unicode filenames. 185 related Node and
13 freshly compiled pure Kotlin/JUnit cases pass; the production gateway now
passes full Release compilation in 1552. It is installed, not device accepted.
Standalone browsing, other mounted providers and citation-graph-only references
remain code gaps; reuse this implementation.

Latest source extension: [private multi-attachment batches](chatgpt-private-attachment-batch.md)
now join up to nine native selections to one prepared-action message, reusing
existing per-file upload owners. Earlier 305 Node/32 Kotlin checks and grouped
build passed. Normal 1598 subsequently passed one real three-file native send
and all file-content reads after separating official ACK from editor cleanup;
228 targeted regression cases passed. [Evidence](reports/chatgpt-media-batch-1596.md).
This completes ordinary new-chat text/PNG/PDF, not other context/format variants.

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
| Private static-image attachment upload | Native JPEG/PNG/WebP handoff, bounded preparation, multimodal upload and dimensions implemented | Normal 1598 ordinary mixed batch passed private upload, accepted native send and actual PNG reading | PNG scope completed; other formats/contexts pending; [contract](chatgpt-private-image-upload.md) |
| Private temporary-chat attachments | New/existing scope, non-library processing and exact text/image association implemented | Normal 1595 new temporary single-text private upload/send, reply, read-only selection and bounded library inventory passed | New single-text scope completed; existing/image/PDF variants pending; [contract](chatgpt-private-attachment-scopes.md) |
| Private new-project attachments | Fresh permission read, scoped text/non-ingest-image upload and library-file ready-store metadata implemented for new project chats | Included in 96 Node cases and grouped Release build; project/library acceptance pending | Published/installed 1541; read-only/ingest-image extensions are source-only below; [exact scope](chatgpt-private-attachment-scopes.md#project-checkpoint) |
| Private existing-project attachments | Fresh membership plus official selected-branch binding; scoped text/non-ingest-image processing and ready-store origins implemented | Included in 96 Node cases and grouped Release build; actual module access and project upload pending | Published/installed 1541; [branch contract](chatgpt-private-attachment-scopes.md#existing-project-branches) |
| Private PDF attachments | Existing byte lease, model-bound create and ordinary/temporary/project association extended to PDF | Normal 1598 ordinary mixed batch passed private upload, accepted native send and actual PDF reading | Ordinary scope completed; temporary/project variants pending; [contract](chatgpt-private-pdf-upload.md) |
| Attachment cancellation without UI-thread I/O | Immediate byte-lease revocation, off-thread file cleanup and stale-read exclusion implemented | All 6 native reader tests passed, including blocked-read/EOF cancellation; grouped Android 33/33 passed | Published/installed 1541; live slow-read cancellation not exercised |
| Private conversation file download authorization and native transfer | Implemented scoped private GET, expiring selections and production Download action | Official current source contract, targeted JS, Release compilation and Android checks passed; device transfer pending | Published/installed 1541; [scope and contract](chatgpt-private-file-download.md) |
| Project/library-linked conversation attachment downloads | Existing native Download action now resolves confirmed project scope and library file metadata before private authorization | 48 focused Node cases passed with synthetic HTTP; actual saved bytes and live scope acceptance pending | Published/installed 1541; [scope extension](chatgpt-private-file-download.md#project-and-library-extension) |
| Private single-conversation deletion | Verified runtime selects the evidenced current flag; confirmation, voice/send exclusion and exact cache invalidation retained | Targeted JS and Release build passed; 1547 production handler returned server acknowledgement for the dedicated synthetic current conversation | Published/installed 1547; personal-current case completed; other selections, variants and rendered confirmation pending; [scope](chatgpt-private-conversation-delete.md) |
| Existing file index and upload receipts exposed through production MCP | Current-document/current-route file descriptors and expiring opaque selections; upload receipt survives unrelated send/skin receipts | Release compilation and 43 targeted Android tests pass; no new network/DOM path | Source-only for grouped APK; [contract](chatgpt-private-conversation-files.md#production-mcp-acceptance-access) |
| Private generated-image library pagination | Private cursor catalog, bounded cache, previous/next and cancellation; v8 adds thumbnail-first loading and on-demand full preview | Installed 1620: native entry/new page 4 ready/viewer/return passed; 52 Node + 7 JVM checks and Release passed | Distinct thumbnail/full transfers and heat benefits remain unverified; [contract](chatgpt-private-image-gallery.md) |
| Independent native file library | Browsing/cache/downloads, rename/soft-delete and composer association implemented | Ordinary saved bytes, native menu, single library attachment and mutations completed through 1589; normal 1591 More/folder/back passed | [Current evidence](reports/chatgpt-library-scopes-1591.md); folder writes/special sources remain open; reuse completed navigation |
| Private images in ingest projects | Existing upload transaction now resolves the current official retrieval gate, reusing the selected-branch runtime and exact project association | 101 targeted Node cases pass, including new/existing project flags, cancellation and production asset-bundle parse | Source-only for grouped APK; [contract](chatgpt-private-attachment-scopes.md#ingest-project-images); actual account gate and indexed-image acceptance pending |
| Private attachments in read-only project chats | Production upload now separates chat-only attachment permission from project file writes, preserving native and official ready-store association | 110 targeted Node cases pass, including new/existing text/image/PDF, exact request scope, stale/cancel guards and asset-bundle parse | Source-only for grouped APK; [contract](chatgpt-private-attachment-scopes.md#read-only-project-chats); actual membership, reply and file placement pending |
| Private common-document attachments | Existing native File action and byte lease admit 87 explicit document MIME types, including Office, structured text, code/scripts, Pages/Keynote and HWP/HWPX; existing project and reservation scope rules reused | 212 attachment Node cases, 60 post-version-bump cases and 11 freshly compiled Kotlin/JUnit policy/reader tests pass; byte integrity, MIME parity, scope and HWP reservation combinations covered | Source-only, adapter 289; [contract](chatgpt-private-document-upload.md); full Android compilation and actual document-reading acceptance pending |
| Private full-conversation public sharing | Standard personal-account transaction, selected-branch binding, native confirmation and validated result implemented | Offline/Release checks passed; 1547 production handler created a synthetic conversation link, listed and revoked it | Published/installed 1547; personal-create case completed; rendered confirmation/Copy/share-sheet and other scopes pending; [contract](chatgpt-private-conversation-share.md) |
| Private project-member conversation sharing | Personal shared-project read-only metadata resolution, official member URL builder, owner/current-conversation binding, 60-second cache and native member-only consent/result implemented; no public or membership write | Included in 131 focused Node cases and nine pure Kotlin tests; long-link protocol receipt boundary verified | Source-only for grouped APK; [scope and pending live acceptance](chatgpt-private-project-conversation-share.md) |
| Private existing public-link management | Scoped native list/Copy/revocation, account-bound cache, consumed selection ticket and complete readback implemented | Offline/Release checks passed; 1545 empty-list and 1547 exact synthetic link list/revoke production-handler cases passed | Published/installed 1547; personal list/revoke completed; rendered menu and other scopes pending; [contract](chatgpt-private-shared-links.md) |
| Direct official-runtime text submission | Native input updates the current official draft through its runtime and invokes that same submit owner, without DOM send-button polling; existing send ledger and stream observer retained | APK 1559 fresh/continuing guest plain-text sends both returned `official_runtime_v1:accepted`, one fixed reply each, no fallback | Published/installed 1559; [live scope](reports/chatgpt-runtime-send-20260908.md); attachments, other contexts and independent HTTP remain separate acceptance gaps |
| Private multipart and same-origin attachment byte routes | Existing create/upload/process owner now supports bounded Azure blocks/commit and the official Estuary FormData path, retaining native association and single send ownership | 148 focused Node cases pass, including exact bytes, concurrency, cancellation, timeout, identity separation and unchanged image/PDF/document/project contracts | Source-only for grouped APK; [contract and live acceptance gaps](chatgpt-private-attachment-byte-transports.md); no new route observed on a device yet |
| Private upload reservation allocation and claim | Current official experiment, bounded native camera/photo/file picker-open prewarm, exact selected-file binding, fresh send-time scope, zero-wait one-shot claim and byte preparation overlap integrated into the existing uploader; temporary scope now uses the evidenced slot default without changing non-library intent | 190 focused Node cases pass, including new/existing temporary text/PDF/image claims, pending-slot legacy behavior, scope/privacy mismatch, pause/reinjection, cancellation/expiry/replacement and existing attachment regressions; six new Kotlin owner tests await grouped execution | Source-only for grouped APK; [contract](chatgpt-private-upload-reservations.md); live picker/allocation/claim and measured speed pending |
| Private existing-library attachment reuse | Current official composer library intent, recognized reuse flags, bounded concurrent SHA-256 lookup, byte-cancellation ownership and one ready-file association integrated into the existing native attachment sender | 212 focused Node cases pass, including reuse/miss/error, no-wait upload winner, reservation/Estuary/multipart cancellation, PDF/image metadata and stale/cancel guards | Source-only for grouped APK; [contract and remaining advanced behavior](chatgpt-private-attachment-library-reuse.md); live gate/reuse and actual latency/heat impact pending |
| Explicit attachment upload-copy choice | Per-selection native choice bypasses reuse/reservations without changing privacy intent or silently falling back | 247 Node/15 JVM checks; normal 1589 native menu, one private text-file upload, exact file-content reply and disposable cleanup passed | Ordinary text scope completed; [receipt](reports/chatgpt-library-mutations-1589.md); additional attachment scopes pending |

Root cause, exact modules, and check results are in
[the history contract](chatgpt-private-history-native-contract.md).
The attachment-index scope and acceptance are in
[the file index contract](chatgpt-private-conversation-files.md).
Request ownership and the confirmed baseline failures are in
[the request lifetime contract](chatgpt-private-request-lifetime.md).
The opt-in research command and its limits are in
[the protocol evidence contract](chatgpt-private-protocol-evidence.md). It is not
a replacement for any missing business protocol below.
Earlier legacy source-location failures are historical. The grouped 2026-09-07
run updated obsolete source contracts to the current owners without skipping
tests; all 1,007 Android tests then passed. This is not device acceptance of
every private transport or a reason to repeat already-verified capabilities.

## Grouped release

### Current 1542 through 1544 acceptance

The [current report](reports/chatgpt-grouped-release-20260907.md) records unified
compilation/tests, three replacement installations, actual private attachment
association and file-content reply, the file-index regression/fix, and the
remaining download-source rejection and 1544 same-origin candidate awaiting
an unlocked-phone saved-byte check. Keep the Goal active and Google deferred;
the complete ChatGPT acceptance gate has not passed.

### Earlier 1541 acceptance

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
and are now implemented in the source-only candidates above. Multipart was still
missing at that checkpoint and is now implemented in the byte-route candidate.
The
subsequent [PDF extension](chatgpt-private-pdf-upload.md) is now implemented and
included in the 96-case focused Node run. Its Kotlin checks subsequently passed
in the 1541 grouped build above; actual runtime model binding remains pending.
The offline implementation itself did not operate the absent phone.

## Protocol gaps

[Bounded segmented image pointers](chatgpt-private-image-download.md#bounded-segmented-pointers-2026-09-08)
now extend both existing download and gallery owners in source (parser 2,
download 13, gallery 3). 161 related offline cases pass; grouped build and real
preview/saved-file acceptance remain pending. Reuse this implementation;
remaining pointer gaps mean other scopes or forms outside its bounded contract.

[Official-runtime generation stop](chatgpt-official-runtime-stop.md) and ordinary
authenticated follow-up continuity passed on APK 1591. It reuses the sender's
committed context and blocks a second writer while stopping. The previous guest
continuity failure still needs its own safe acceptance scope. This is not
independent Android HTTP dispatch. Older source-only cells below are historical
checkpoints; the grouped delivery section controls their build/install status.

| Area | Existing usable path | Actual remaining private work |
|---|---|---|
| Text send/regenerate | Native send ledger and streaming observer; [official current-draft runtime transaction](chatgpt-official-runtime-text-submit.md) and [guarded runtime regeneration](chatgpt-official-runtime-regeneration.md); guest fresh/continuing plain-text sends accepted on 1559 without a DOM send click | Accept owned-attachment handoff/cleanup, other contexts and original-parent retry. Independent fresh proof-bound private dispatch and regeneration remain unverified; runtime acceptance is still official-page authority, not independent HTTP success. Do not replay captured proof headers or repeat accepted guest sends without a regression. |
| Model/effort/tools/temporary mode | Native presets/cache, [search/image private state](chatgpt-private-composer-tools.md), [model/effort/version/tier](chatgpt-private-model-state.md), [additional model catalog](chatgpt-private-model-catalog.md), and [temporary-chat transaction](chatgpt-private-temporary-chat.md) implemented | Guest Search toggle completed on 1565/adapter 302: private accepted on/off in 165/180 ms with no document reload; reuse without repeat testing. Image, model/effort/temporary and account-scope cases remain unaccepted. These callbacks are not independent generation/privacy POSTs, and local readback does not prove server preference persistence. |
| Attachment upload | Earlier small-text private upload verified; production text-file delivery passed on 1541; native byte handoff and ordinary/temporary/writable-project image/PDF transactions packaged; ingest/read-only projects, [87 explicit common-document MIME types](chatgpt-private-document-upload.md), [multipart/Estuary byte routes](chatgpt-private-attachment-byte-transports.md), [ordinary/temporary picker prewarm and reservation claim](chatgpt-private-upload-reservations.md), [intent-bound library reuse and pre-send upload-copy choice](chatgpt-private-attachment-library-reuse.md) now implemented in source | Confirm integrated private-route provenance, image/PDF/Office reading, runtime access, native picker/copy menu, byte/reservation/reuse routes, read-only file placement and temporary/project library behavior. Unknown/ambiguous MIME, media/archive categories and cloud-native references remain unimplemented. Persistence follows the existing official option, never deduplication convenience. Production file delivery is not a private-route pass. See [upload contract](chatgpt-private-attachment-upload.md), [PDF extension](chatgpt-private-pdf-upload.md) and [scope contract](chatgpt-private-attachment-scopes.md). |
| Images | Native gallery/previews/cache; private generated-catalog pagination and scoped previews implemented; tool-state selection and the existing official-runtime submit path preserve official generation preparation | Accept the [private gallery candidate](chatgpt-private-image-gallery.md), and verify actual image generation through the native Tools and send controls. The inspected official `EOn` reads the same selected tool signal during runtime submit; do not create a duplicate sender merely because standalone generation POST remains unverified. Remaining pointer scopes and independent fresh-proof dispatch are separate gaps. Do not count source evidence as live API success. |
| Share/delete/conversation files | Native pin/rename/archive/move; private file index plus ordinary/project/library-linked, [simple and bounded parameterized image-pointer](chatgpt-private-image-download.md), [imported connector-copy](chatgpt-private-connector-file-download.md) and [shared-library attachment and metadata-only reference download candidates](chatgpt-private-shared-library-download.md), now with in-app progress, request-bound cancellation and [durable crash cleanup](chatgpt-private-download-recovery.md); guarded flag-selected private delete and current-chat reset; [ordinary full-conversation private share candidate](chatgpt-private-conversation-share.md) | Verify saved-file downloads, in-app progress/cancel, Android crash cleanup, live flag binding, current/noncurrent deletion and share creation/update. The [mounted-file/export extension](chatgpt-private-mounted-file-download.md) passes 185 related Node and 13 pure Kotlin cases; grouped Android and device checks remain pending. Accept the implemented [standalone library browser](chatgpt-private-library-browser.md); complete its remaining variants, other mounted providers, citation-graph-only references, connector-only cloud-reference and remaining image-pointer scopes/path forms, plus remaining sharing variants. Binary process-death resume remains unsupported. System distribution alone is not official share creation. |
| Google direct send (last) | Native cache and private response observer; official submit | Deferred until remaining ChatGPT code is ready and its grouped phone acceptance passes. Then reproduce the submit contract and transaction ownership; observed reply endpoints do not imply a working private sender. |

An unknown protocol remains a documented code gap. No guessed endpoint, fake
success, or automatic write replay should be added merely to make this table
look complete. Existing UI stays usable while each replacement is implemented.

The 2026-09-07 runtime attachment extension (adapter 279) passed 196 focused Node
cases and retains Android's ban on text-template replay for attachment commands.
It uses the official prepared action and one sender-owned ready-file lease, not
an independent generation POST. Grouped build, live project/file dispatch and
resource/latency acceptance remain pending; do not repeat this source integration.

The connector-copy download extension (adapter 280, commit `6581991c8`) passed
111 focused Node cases, including the production bundle parse. It reuses the
ChatGPT file ID and scoped authorization, never the external connector URL.
Grouped device transfer and saved-byte verification remain pending. The separate
binary-route candidate below supersedes the indexed shared-library code gap;
parameterized pointers were a gap at that checkpoint. Adapter 288 now implements
the [bounded parameterized-pointer extension](chatgpt-private-image-download.md#parameterized-pointers)
(`0ed2edcf2`), with 118 focused Node cases and six wiring/bundle checks passing.
It reuses the native download action and scoped lease; grouped saved-byte
acceptance, path-bearing references and the other listed scopes remain pending.

The shared-library attachment candidate (adapter 281, download module 5) passed
147 focused Node cases, including the production bundle parse. It streams the
official binary route into a one-use native storage transaction with ordered
packets, bounded time/size, cancellation and saved-versus-queued receipts. Its
five new Kotlin transfer tests and one command-lifecycle test await the grouped
Android build. Real response bytes, redirects/CORS, storage and notifications
remain unverified on the phone; source tests do not satisfy the ChatGPT gate.
See its [contract and exact remaining gaps](chatgpt-private-shared-library-download.md).

The subsequent adapter 282/download module 6 checkpoint adds native in-app
progress, collapse/reopen and exact-request cancellation without notification
permission. It removes the old independent toast-result poll and reuses one
download owner for UI/MCP. The 149-case Node run passes; the two new cancellation
cases fail on the baseline module. Ten additional Kotlin tests are written,
not run. Browser navigation and wireless ADB connection timed out in this
attempt, so grouped Android compilation and real file/UI acceptance remain
pending. This closes a source UI gap, not the private-protocol acceptance gate.

## Sharing protocol checkpoint

The earlier missing-consumer checkpoint below is superseded by the 2026-09-07
discovery of the official lazy modal and full-conversation create/publish
protocol. The ordinary personal-account path is now wired to production native
UI. Adapter 283/share modules 2 also implement the current-runtime-selected
redesigned `/share/v2/create` transaction, without the legacy PATCH. Variant is
bound through dispatch and cache reuse; unknown or changed state cannot trigger
an alternate writer. The combined 88-case Node run passes, with all 19 new
variant cases reproduced as failures on the preceding implementation. See the
[current protocol, source hashes and remaining sharing scopes](chatgpt-private-conversation-share.md).
This is source implementation evidence, not a live public-link acceptance pass.
The browser navigation retry still timed out and ADB reported no device in this
attempt. Grouped Android compilation and real response/runtime confirmation are
pending. Adapter 284 subsequently adds personal shared-project member links,
member-only native consent and validated long receipts: 131 combined Node and
nine pure Kotlin cases pass. See the [member scope and evidence](chatgpt-private-project-conversation-share.md).
Guest, workspace, private-project public sharing, temporary and
management/revocation scopes remain code gaps, not merely untested parts.

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
