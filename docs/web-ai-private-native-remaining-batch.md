# Remaining private-native batch

Current implementation audit: 2026-09-06. This is a work list, not a declaration
that every private protocol has been reproduced. Reuse completed capabilities in
[the capability matrix](web-ai-private-transport-capability-matrix.md).

## Workflow

Implement coherent modules with targeted checks and separate commits. Do not
publish an APK for every small correction. Use one grouped install/acceptance
round after the candidate batch is ready. Existing proven native audio,
subtitles, dictation, read-aloud, directory cache, and mutations are not repeated
research. System alternatives remain explicit choices, not silent replacements
for website functionality. Persistent WebView identity remains intentional.

## Current batch

| Work | Code | Verification | Delivery |
|---|---|---|---|
| Private history to native wire format, citations, file/image descriptors | Implemented | JS and shared Android fixture passed | Published/installed 1540; device UI acceptance pending |
| Content-only refresh preserves current composer/voice state | Implemented | Targeted Android tests passed | Published/installed 1540; device UI acceptance pending |
| Bounded/coalesced image requests and no false empty-library success | Implemented | Targeted JS passed | Published/installed 1540; device UI acceptance pending |
| Private conversation attachment index, cache and native file sheet | Implemented | Shared JS/Android contract and targeted production tests passed | Published/installed 1540; device UI acceptance pending |
| End-to-end private read deadlines, body limits and late project response isolation | Implemented | Lifecycle and existing JS consumer suites passed | Published/installed 1540; device acceptance pending |
| Bounded request-shape capture through native MCP, reusing the page observer | Implemented diagnostic only | Node/Android checks passed; actual reservation JSON and conversation SSE capture observed | Published/installed 1540; telemetry-budget follow-up is source-only |
| Reservation responses cannot prematurely release attachment sends | Implemented regression correction | Node red-to-green, 12 Android tracker tests and file-content smoke contract passed | Source-only; grouped APK and synthetic-file acceptance pending |
| Private file create/blob upload/process transaction | Implemented transport and native byte/store integration for one plain-text file in an empty ordinary new-chat or existing-chat composer, with confirmed conversation scope | One real 78-byte private upload processed; earlier Release compilation and 26 Android tests passed; adapter 269 JS checks passed; integrated device acceptance pending | Adapter 269 source candidate; no integration APK release yet; [scope and contract](chatgpt-private-attachment-upload.md) |
| Private static-image attachment upload | Native normalized JPEG/PNG/WebP handoff, bounded image preparation, multimodal private upload and ready-store dimensions implemented | 60 focused Node cases passed with synthetic decoder/HTTP; grouped Android and real image-reading acceptance pending | Adapter 270 source candidate; [image contract and acceptance](chatgpt-private-image-upload.md) |
| Private temporary-chat attachments | New/existing temporary scope, non-library processing intent and exact text/image ready-store association implemented | 67 focused Node runner cases passed with synthetic HTTP/decoder; grouped Android and live upload/library checks pending | Adapter 271 source candidate; [scope contract and remaining project work](chatgpt-private-attachment-scopes.md) |
| Private new-project attachments | Fresh permission read, scoped text/non-ingest-image upload and library-file ready-store metadata implemented for new project chats | 75 focused Node cases passed; grouped Android and project/library acceptance pending | Adapter 272 source candidate; existing project branches, read-only and ingest-image flag still open; [exact scope](chatgpt-private-attachment-scopes.md#project-checkpoint) |
| Private conversation file download authorization and native transfer | Implemented scoped private GET, expiring selections and production Download action | Official current source contract, targeted JS, Release source compilation and 22 Android tests passed; device transfer pending | Adapter 266 source candidate; [scope and contract](chatgpt-private-file-download.md) |
| Private single-conversation deletion | Partial: evidenced legacy PATCH, current/noncurrent native confirmation, voice/send exclusion and exact cache invalidation | 12 deletion JS cases, existing regressions, Release source compilation and 90 Android tests passed; final send guard rechecked in the affected 8-test suite; live deletion pending | Adapter 268 source candidate; [scope and remaining work](chatgpt-private-conversation-delete.md) |

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

Next complete the attachment prepare/upload/finalize contract and its composer
association, reusing this real evidence, the existing synthetic fixture and
bounded MCP command. The subsequent [reservation regression](chatgpt-private-protocol-evidence.md#reservation-completion-regression)
invalidates generic HTTP completion as upload proof; include that correction in
the next grouped candidate before accepting attachment delivery. First confirm a healthy transport and preserve the current
draft, conversation and voice state. Do not rebuild this unchanged candidate,
add another probe framework, guess an endpoint, or repeatedly restart the app
because the debugging connection is unavailable. The Goal is not complete.

## Protocol gaps

| Area | Existing usable path | Actual remaining private work |
|---|---|---|
| Text send/regenerate | Native send ledger and official transaction; streaming observer | Fresh proof-bound private dispatch is not verified. Do not replay captured proof headers or declare official fallback a private POST success. |
| Model/effort/tools/temporary mode | Native presets/cache and official controls | Apply the chosen state through a confirmed private contract; cached menu labels alone are not proof of server selection. |
| Attachment upload | Verified small-text private upload; native byte handoff, store association, ordinary/temporary and writable new-project scopes, static-image transaction connected in source | Accept integrated text/images including temporary/project library behavior, then finish existing project branch/permission variants, ingest images, PDF/multipart. No integrated-device pass is claimed. See [upload contract](chatgpt-private-attachment-upload.md), [image extension](chatgpt-private-image-upload.md) and [scope contract](chatgpt-private-attachment-scopes.md). |
| Images | Native gallery/previews/cache; official creation and library sync | Confirm private library pagination and generation transaction. Download queue improvements do not replace these endpoints. |
| Share/delete/conversation files | Native pin/rename/archive/move; private file index/download candidate; guarded legacy private delete and current-chat reset | Verify ordinary-file download and current/noncurrent deletion. Complete flagged delete endpoint selection, project/library/image download scope, and official sharing. Do not substitute system sharing for official features. |
| Google direct send | Native cache and private response observer; official submit | Reproduce the current submit contract and transaction ownership; observed reply endpoints do not imply a working private sender. |

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
