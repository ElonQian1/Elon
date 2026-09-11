# Project citation metadata ownership

## Status

Project uploaded-TXT citation download is completed/default-enabled and verified
on normal 1657 / owner v27 / adapter 339; see [the acceptance](#normal-1657-acceptance).
The 1656 metadata 404 below is historical reproduction evidence, not current
status. Personal TXT citation and ordinary gallery download remain completed
separately; grouped/cloud/PCA and other-owner scopes are not implied.

## Reproduction on normal 1655

- An existing project citation returned HTTP 404 at
  `/backend-api/files/{id}/simple`, before download authorization.
- The original user attachment control returned authorization HTTP 200 and
  `download_queued`, but native saved bytes were not confirmed. A successful
  authorization is not a successful download.
- A fresh conversation in the same synthetic project uploaded one fixed TXT
  and sent one request. Private association and `official_runtime_v1:accepted`
  were observed. Its sole reply quoted the exact fixture marker; elapsed time
  was 26,958 ms for the whole response, not first-token latency.
- Read-only reconciliation found two file rows, one assistant citation, with
  matching project/conversation and a fresh index. Its native Download action
  again returned metadata 404, with zero received bytes. No message was resent.
- Original conversation/draft and awake setting were restored in these runs.

Logs: `project-attachment-control-1655-20260911-112259-264`,
`project-citation-fresh-1655-canonical-20260911-114044-218`,
`fresh-citation-inspect-1655-20260911-114320-167`, and
`project-citation-fresh-1655-download-20260911-114521-950`.

## Official source comparison

Retained public assets from `runtime-20260911-b`:

| Asset | SHA-256 |
|---|---|
| `8b34dbc2-fpy4mlfnxc115y6k.js` | `c4b74136b4efd5255fd91e9c0f2f9c382212ffc28f5ecee7aaa564ed05e36cec` |
| `conversation-small-ft205i7yqa6zc2nj.js` | `a89952420338983e104e94be5fae8e9ae7a4169b9ea8a4ed30e639641e14f456` |
| `4813494d-c6b4nsqqwi13e6rd.js` | `41e6e38589707e7c6ff5d181afa2193dda17f605dfe256224af09e6c85cfa096` |

Composer `V3t` passes no `gizmoId` for persisted message attachments to the
file-citation preview; staged files use their explicit file ownership. The
conversation's `C3n/w3n` preview entry similarly obtains effective preview
ownership from its explicit `gizmoId` or `filePreviewAuth`, separately from the
server conversation ID. The metadata loader accepts those separate values.
None of these inspected call paths infer file ownership from the route alone.

Our citation download previously collapsed current-conversation project and
explicit file project into one value, then supplied it to the metadata request.
The candidate keeps these values separate: citation metadata uses explicit
reference ownership only; `conversation_id` remains present. Existing metadata
validation, effective-library ownership resolution and conversation-scoped
download authorization are unchanged. It does not retry 404 without scope,
bypass permissions, export credentials, or change uploads/audio.

This is an evidence-backed parameter correction, not yet proof that this
explains every project-file failure or that grouped/cloud/PCA variants work.

## Normal 1656 and direct authorization

Source `61359c4d6cfecc41dd40baa6a7c7ecfaeeb5d662` built and published normal
`v1.1.1656` (1656), SHA-256
`ffe8a9138a504cc25c2dd2a973384b1cca85a7c83e2d708e2b9d27a8d365767d`.
The release script updated the trusted Xiaomi without clearing data; package,
adapter 338 and authenticated state were reread. Release log
`citation-scope-release-338-20260911-115445-199` passed in 494.9 seconds.

The first device case stopped before Download when another application became
foreground. A bounded retry reused the existing fresh fixture with zero sends
and one native Download. It still returned metadata 404 and zero bytes;
conversation/draft/awake were restored. Log
`project-citation-1656-native-retry-20260911-120542-614`. Thus v26 did not fix the
observed project citation failure.

Further source tracing identified shared export `nm`, local `fDt`, called from
the current conversation preview `w3n`. It skips metadata when both explicit
file project and library identity are absent. That differs from the legacy
`FileCitationPreviewSheet`, which always requests metadata. Composer `MW` routes
concrete `file_` and `file-` IDs toward the generic preview when its feature is
enabled; shared `jX/TMe` recognizes both forms.

Candidate v27 applies that known direct authorization only to project-chat
citations with neither explicit file project nor library identity. It sends
the concrete file ID and `check_context_scopes_for_conversation_id`, without
inventing file project ownership. Explicit ownership still gets metadata
validation. Previously accepted personal citations keep their existing path.
Authorization errors do not retry, drop conversation scope, or start bytes.
This is the initial scoped authorization request, not a 404 permission bypass.

The direct-path regression failed on v26, then the related 183 cases passed
on v27. Additional rejection tests cover 403/404/429/500 without replay.
Logs: `citation-direct-authorization-red-20260911-121008-848` and
`citation-direct-authorization-green-20260911-121032-166`.
The final seven-file set including rejection cases passed all 187 tests:
`citation-direct-authorization-final-20260911-121136-001`.

## Verification

### Normal 1657 acceptance

Source `2f761bc38d8dffaf9b0981628ea6d0cea823a9f1` built and published normal
`v1.1.1657` (1657), SHA-256
`c9a5239cb53b411850e74e0f3ded693647e8a29b1c592df99c7752f7618fdbf1`.
The release script updated the trusted Xiaomi without clearing data. Endpoint
version/source/hash and installed adapter 339/authenticated state were reread.
Log `citation-direct-release-339-20260911-122114-843` passed in 433 seconds.

The first acceptance stopped at a failed file-list read, before any Download.
Two retained list receipts reported `files_read_failed`; its root cause is not
established. It restored the conversation and awake settings. The outer one-off
wrapper initially failed to propagate the child script's exit status, so its
runner-level pass is not a business pass. The emitted report correctly says
`passed:false`. The wrapper was corrected before the bounded retry.

`project-citation-1657-index-retry-20260911-123122-806` passed: zero uploads/sends,
one actual native Download on the assistant TXT citation, exactly one new file,
78 bytes, SHA-256
`75e2ed9bfe5772c9918e552ed07c2c0e689e7039367c81bb6906c63e396fa1f3`.
Both the native handoff and saved-byte/hash checks passed; original conversation,
draft and awake settings were restored. No private content was exported.

This retry temporarily observed requests before the file-list step; background
website traffic filled the bounded probe (19 dropped records), so the log does
not independently record the final authorization HTTP status. Do not claim it
does. The native command receipt and new verified file prove the consumer result.
The retained harness keeps its normal download-only observation window.

The project uploaded-TXT citation scope of
`android_chatgpt_private_file_citation_download_v1` is **completed**, default
enabled, and device verified on 1657. Reuse it without another upload or research
cycle. This does not complete grouped/cloud/PCA citations, every explicit-owner
variant, or explain the initial file-list read failure.

The focused regression first failed three cases on v25: personal-library
citations inside project chats (false/null project flag) and a metadata endpoint
rejecting inherited project ownership. Explicit file ownership remains covered.
After v26, the related seven-file Node set passed all 183 cases, with no skips
or cancellations. The PowerShell citation acceptance contract also passed.
Logs: `citation-metadata-scope-red-20260911-115008-771` and
`citation-metadata-scope-green-20260911-115044-026`.

## Acceptance harness corrections

The website canonicalizes project display slugs. A strict cached-URL equality
check incorrectly rejected its ready empty project composer. The harness now
checks exact project identity, official origin and an empty query/fragment.
Cached directory pages are bounded locators only; live synthetic messages and
file metadata still verify the target before writes. A refreshed directory may
move the fixture beyond the first page, so at most four cached pages are read.

The file UI can initially show cached data while a refresh is outstanding.
Acceptance waits boundedly for the existing refresh instead of sending another
refresh. Fresh project fixtures have a local checkpoint and an explicit reuse
mode; a retained checkpoint prevents a second upload/send. `FixtureCheckpoint`
allows retaining this navigation-only artifact outside a disposable worktree.
Downloads still use the native UI button and require saved-byte/hash evidence.

The byte sink has two legitimate outcomes: same-origin transfers report saved,
while authorized external URLs are handed to Android DownloadManager. The
previous UI waiter rejected that queued state immediately, before checking
storage. It therefore did not prove that the original attachment control failed
to transfer. A later read-only check found six retained fixed TXT fixtures with
matching hashes; it cannot attribute those files to a particular earlier run.
The citation harness now explicitly accepts either confirmed handoff or saved
receipt, then waits at most 25 seconds for exactly one newly created file with
the expected 78 bytes and SHA-256. Queued alone never passes. Other existing
acceptance callers retain the strict saved-state waiter. The PowerShell contract
and Java semantic harness compile passed after this correction.
