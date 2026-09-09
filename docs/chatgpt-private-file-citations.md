# Private file-citation downloads

## Status and boundary

Capability: `android_chatgpt_private_file_citation_download_v1`.
Status: implemented; offline protocol/owner checks passed; grouped APK build and
real production download acceptance pending. Not marked `completed`.

This extends the existing conversation file index and download owner. It is not
a new uploader, downloader, background poller or guessed cloud-provider API.
The existing published 1620 APK does not contain this source batch.

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

## Native integration

- `chatgpt_web_private_file_citation.js` v2 accepts explicit files and concrete
  file IDs in `grouped_webpages`, `grouped_webpages_v2`,
  `grouped_webpages_model_predicted_fallback` and cite-map values. It preserves
  library/project identities and rejects conflicting or unknown download scopes.
  Classification follows the official category-first rule; attribution or a
  `file://` URL alone still cannot supply a missing ChatGPT file ID.
- Grouping follows `kpr`: fallback items are used only when the primary array
  is empty; supporting websites inherit the parent category/retrieval origin;
  the first source for a URL wins. Deleted/PCA children remain excluded even
  when their parent is ordinary. Cite-map keys are not download URLs.
  Traversal is bounded to 20 source candidates, without recursive metadata walks.
- History projection v8 appends citation rows after existing image/attachment,
  shared and mounted rows. File and library IDs deduplicate against attachments.
  Selected-branch and hidden-message rules, index size and truncation remain
  controlled by the existing projection. Nested overflow marks the native index
  as truncated too. Raw references stay in the page.
- Download owner v15 revalidates the raw citation and uses the existing scoped
  file authorization, library metadata check, expiring selection handle,
  native transfer and save receipt. Account/document changes invalidate it.
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
download, shared references, mounted download, image download and connector-copy
tests passes 129 runner cases (the history script additionally checks its 12
assertion cases). The changed production asset catalog also compiles as one
JavaScript bundle. Source checks do not measure network latency or temperature.

The grouped/cite-map extension first failed 14 of 28 focused runner cases on
the unchanged owners, then passed all 28 and the 129-case adjacent set after
implementation. Integration cases cover opaque native selection, the existing
scoped download request/save receipt, library/project checks, attachment-first
ordering, nested truncation and invalidating old handles after a deleted-source
refresh. The DOM getter throws in these fixtures; HTTP and native receipts are
synthetic. This is not real-account or installed-APK acceptance.

Use an existing synthetic conversation with an assistant file citation after
the next grouped APK build. In production native UI, open Conversation files,
select the citation, download it and verify the save receipt and actual bytes.
Keep the visible conversation/draft unchanged. Then exercise one library/project
citation and a grouped/cite-map file if present; do not create private user
content merely for a fixture.
Phone was absent from the bounded ADB inventory for this source batch.
