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
Its `kpr` maps `content_references` entries of type `file` to file-source items;
`c2`, `p6i` and `g6i` classify files and resolve the file ID, name, MIME and
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

- `chatgpt_web_private_file_citation.js` v1 parses only explicit `type: file`
  content references with a concrete, bounded ChatGPT file ID. It preserves
  library/project identities and rejects conflicting or unknown download scopes.
- History projection v7 appends citation rows after existing image/attachment,
  shared and mounted rows. File and library IDs deduplicate against attachments.
  Selected-branch and hidden-message rules, index size and truncation remain
  controlled by the existing projection. Raw references stay in the page.
- Download owner v15 revalidates the raw citation and uses the existing scoped
  file authorization, library metadata check, expiring selection handle,
  native transfer and save receipt. Account/document changes invalidate it.
- No new HTTP route, DOM readiness wait, automatic navigation, write replay,
  Cookie export or proxy change is introduced. The original citation/official
  preview routes remain available for unsupported shapes.

Not implemented here: grouped/cite-map references, URL-only cloud documents,
artifact-specific export, or conversation-context/PCA citation graphs. The
latter have additional deletion and source-mask state; incomplete metadata
must not be flattened into downloadable attachments. Do not broaden the parser
without authoritative state and protocol evidence.

## Verification and next acceptance

`node --test` on file-citation, citation-integration, history projection, file
download, shared references, mounted download, image download and connector-copy
tests passes 114 runner cases (the history script additionally checks its 12
assertion cases). The changed production asset catalog also compiles as one
JavaScript bundle. Source checks do not measure network latency or temperature.

Use an existing synthetic conversation with an assistant file citation after
the next grouped APK build. In production native UI, open Conversation files,
select the citation, download it and verify the save receipt and actual bytes.
Keep the visible conversation/draft unchanged. Then exercise one library/project
citation if present; do not create private user content merely for a fixture.
Phone was absent from the bounded ADB inventory for this source batch.
