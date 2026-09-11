# Project citation metadata ownership

## Status

Candidate correction: download owner v26 / adapter 338. Offline checks passed;
normal APK build and device acceptance are pending. This is not a completed
project citation download. Personal TXT citation and ordinary gallery download
remain completed separately.

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

## Verification

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
