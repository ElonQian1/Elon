# Private file-citation downloads

## Status and boundary

Capability: `android_chatgpt_private_file_citation_download_v1`.
Completed scope: personal uploaded TXT citation, default-enabled and
device-verified on normal 1651 / adapter 334 / download owner v23. The actual
native Download action saved 78 bytes with the exact expected SHA-256; the
original conversation/draft were restored. No message was resent. Reuse this
scope; 126 related tests pass. Project/grouped/cloud/PCA variants remain separate.
[Acceptance and nullable-metadata fix](reports/chatgpt-runtime-bindings-20260911.md#normal-1651-acceptance).

Earlier delivery: citation v3 / projection v9 / download owner v18
(source `7b78394da`) passed offline checks and is included in normal 1640 / adapter
325 with the earlier metadata/scope correction. Grouped Release/install passed;
production citation download was unverified at that point. The scoped 1651
result above supersedes that pending label, not other variants.
[Earlier evidence](reports/chatgpt-runtime-bindings-20260910-b.md#grouped-normal-release-1640).

This extends the existing conversation file index and download owner. It is not
a new uploader, downloader, background poller or guessed cloud-provider API.

## Current resolver correction

Two gaps in the original parser are corrected without changing the private
metadata/authorization requests or native byte sink:

- The hash-verified conversation bundle's `p6i -> c6i -> g6i` resolves a missing
  explicit file ID from the decoded final pathname component of a `file://` URL.
  Search is appended before ID validation, so query-bearing/unknown identities
  are not silently reduced to an unscoped file. Only a bounded concrete
  `file-`/`file_` identity is admitted; invalid explicit IDs are not rescued.
  The URL itself is never fetched and HTTP cloud attribution cannot supply an ID.
- Its `GFi` distinguishes an empty context-citation metadata array from a
  nonempty context graph. Empty arrays with no status no longer suppress ordinary
  file references. Nonempty/malformed graphs, marker/unknown statuses and
  deleted/PCA references remain excluded; this does not implement PCA access.

The new path uses the existing opaque native handle, attachment-first
deduplication, project metadata and conversation-scoped authorization. Module
versions advance so an already installed download owner does not retain the old
parser on reinjection. Identity, audio and the request protocol are unchanged.

Both defects were reproduced first: 4/53 failing URL cases, then 2/55 failing
empty-context cases. The final related suite passes 216 cases with zero failures,
skips or cancellations, including production asset assembly and native download
packet/ACK fixtures with a throwing DOM getter. Log:
`citation-resolver-final-20260910-202016-901` (terminal pass, 1.9s).
These are source/integration fixtures, not actual Android saved bytes. This
small batch was grouped with the runtime correction in normal 1640, not built separately.

## Verified public source

The retained official conversation bundle `conversation-small-owrec55n6vm0ekcc.js`
has SHA-256 `7973d518b083f0f3e23905a279ed019378481bdbdd10fc0196afe9fc7b3b7d35`.
Its `kpr` maps explicit files, grouped webpage items and cite-map values into
URL-deduplicated sources. `c2`, `p6i` and `g6i` classify files and resolve the file ID, name, MIME and
optional library identity. The shared bundle `4813494d-o593jrji51wy4azk.js`
(SHA-256 `48563cd22f0dafe6c0b89220348fa3add81ff3abb82a62ed9d68a04d569cc375`)
uses `PXe` to distinguish `file_`/`file-` identities.

On 2026-09-10, the public lazy preview module
`https://chatgpt.com/cdn/assets/98ca14f9-ojmm0jfzj01rbbfw.js` was also retrieved:
8126 bytes, SHA-256
`3d9f47f6aa6d18b2681e8d6bfa57ae5a4e5961ecfff9389beef4a0376758f391`.
Its `FileCitationPreviewSheet` obtains file metadata, resolves the effective
project and then requests a preview/download URL. A cloud source URL is
attribution, not proof of downloadable bytes. These are source observations,
not a successful real-account request for the new citation path.

The preview imports shared `Pf` (`cEt`) for
`GET /files/{file_id}/simple`, with `gizmo_id` and `conversation_id`, even when
the original citation omits a library ID. Shared `Nf` (`OX`) resolves the
effective project from that metadata: personal library files clear the requested
project; project library files use their actual project; non-library files keep
the requested project. Conversation `hbt` (`o1n`) supplies
`checkContextScopesForConversationId` to URL authorization. These exact symbols
and the three source hashes were rechecked on 2026-09-10. Our explicit download
keeps `download_intent=true`, not the preview-only `show_inline` behavior.

## Native integration

- `chatgpt_web_private_file_citation.js` v3 accepts explicit files and concrete
  file IDs in `grouped_webpages`, `grouped_webpages_v2`,
  `grouped_webpages_model_predicted_fallback` and cite-map values. It preserves
  library/project identities and rejects conflicting or unknown download scopes.
  Classification follows the official category-first rule; a `file://` pathname
  can supply a concrete ChatGPT ID, but cloud attribution cannot.
- Grouping follows `kpr`: fallback items are used only when the primary array
  is empty; supporting websites inherit the parent category/retrieval origin;
  the first source for a URL wins. Deleted/PCA children remain excluded even
  when their parent is ordinary. Cite-map keys are not download URLs.
  Traversal is bounded to 20 source candidates, without recursive metadata walks.
- History projection v9 appends citation rows after existing image/attachment,
  shared and mounted rows. File and library IDs deduplicate against attachments.
  Selected-branch and hidden-message rules, index size and truncation remain
  controlled by the existing projection. Nested overflow marks the native index
  as truncated too. Raw references stay in the page.
- Download owner v16 revalidates the raw citation and always resolves its file
  metadata, not only citations already carrying a library ID. It then uses
  `check_context_scopes_for_conversation_id` and the effective project for
  authorization, or the existing personal-library byte owner. Metadata failures
  do not fall through to an unscoped request. The bounded metadata read, expiring
  selection handle, native transfer and save receipt are reused; no new cache
  or download owner is introduced. Account/document/route changes invalidate it.
- Explicit cancellation survives an aborted JSON request as `download_cancelled`,
  rather than `download_prepare_failed`. The request timeout and overall prepare
  deadline remain failures, not user cancellations.
- No new HTTP route, DOM readiness wait, automatic navigation, write replay,
  Cookie export or proxy change is introduced. The original citation/official
  preview routes remain available for unsupported shapes.

Not implemented here: URL-only cloud documents, artifact-specific export, or
conversation-context/PCA citation graphs. The
latter have additional deletion and source-mask state; incomplete metadata
must not be flattened into downloadable attachments. Do not broaden the parser
without authoritative state and protocol evidence.

## Verification and next acceptance

`node --test` on file-citation, citation-integration, history projection, file
download, library download, shared references, mounted download, image download
and connector-copy tests passes 203 runner cases (the history script additionally checks its 12
assertion cases). The changed production asset catalog also compiles as one
JavaScript bundle. Source checks do not measure network latency or temperature.

The grouped/cite-map extension first failed 14 of 28 focused runner cases on
the unchanged owners, then passed all 28 and the 129-case adjacent set after
implementation. Integration cases cover opaque native selection, the existing
scoped download request/save receipt, library/project checks, attachment-first
ordering, nested truncation and invalidating old handles after a deleted-source
refresh. The DOM getter throws in these fixtures; HTTP and native receipts are
synthetic. This is not real-account or installed-APK acceptance.

The v16 metadata/scope correction first failed 17 of 26 focused runner cases on
v15. The expanded set then found an explicit-cancel receipt defect (200/201);
after correcting it and adding both timeout boundaries, all 203 cases passed
with zero skips/cancellations. Evidence:
`citation-context-related-final-20260910-115215-587` (terminal pass, 1.8 s).
Coverage includes missing library identity, personal/project resolution,
metadata HTTP errors, malformed flags/IDs, owner invalidation and no download
after cancellation. This does not prove other cloud/PCA protocols or real bytes.

Use an existing synthetic conversation with an assistant file citation on the
installed adapter 325. In production native UI, open Conversation files,
select the citation, download it and verify the save receipt and actual bytes.
Keep the visible conversation/draft unchanged. Then exercise one library/project
citation and a grouped/cite-map file if present; do not create private user
content merely for a fixture.
Wireless ADB was available on 2026-09-10, but the phone remained locked/asleep
after the grouped build and installation. No real-account citation download was
run for v16; see the [delivery evidence](reports/chatgpt-native-regeneration-20260910.md#normal-1630-grouped-delivery).
