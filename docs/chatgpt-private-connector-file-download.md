# Imported connector copy downloads

Capability candidate: `android_chatgpt_private_connector_copy_download_v1`.
Status: **implemented, offline verified, grouped APK/device acceptance pending**.
Implementation commit: `6581991c8`; download module 4, adapter 280.

## Official source evidence

The 2026-09-06 public assets were rechecked by SHA-256 on 2026-09-07:

- `/cdn/assets/conversation-small-hiw4wce20lu6te81.js`, SHA-256
  `296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`:
  `Gar` serializes an uploaded connector copy's file spec as an attachment with
  `id`, filename, MIME and optional `context_connector_info`. That attribution
  contains `context_connector`, `source_url`, `synthetic_extension` and `type`.
  Shared-library references take a different branch; attribution alone does not
  turn an uploaded file copy into such a reference.
- `/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js`, SHA-256
  `9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`:
  `C4t` requests download authorization using the attachment ID, effective
  project and selected conversation scope. It downloads the resulting
  `download_url`, not the connector's source URL. `jW` matches image pointers
  to attachment metadata; `A5t` gives the source URL to the image viewer but
  passes the ChatGPT file ID and conversation/library scope to its downloader.
- `/cdn/assets/4813494d-hrplraurzfyvxb10.js`, SHA-256
  `89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0`:
  `fEt`/`dEt` provide the existing scoped signed-URL contract documented in
  [private file downloads](chatgpt-private-file-download.md).

These are official source observations, not a newly captured authenticated
request or a verified saved-file transfer.

## Native integration

Previously `target()` rejected every attachment with `context_connector_info`,
including a normal uploaded copy with a usable ChatGPT file ID. Version 4
accepts the recognized attribution shape and reuses the existing selected-file
registry, same-origin authorization and one-use native download handoff.

Ordinary files retain `conversation_id`; project/library files and image
pointers retain `check_context_scopes_for_conversation_id`. Library metadata
still resolves the effective project before authorization. There is no broader
retry without scope when a permission check fails.

Only the immutable ChatGPT file ID and existing scope fields enter the
page-local selection. Connector attribution and source URLs do not enter the
native file index, authorization query, native download handoff or receipts.
The source URL is never fetched, including after a 404 or permission failure.
Cookie and authorization headers remain in the identity WebView.

Recognized copies may be ordinary, project-linked, library-linked or a simple
image pointer with matching attachment metadata. Account/document/route changes,
expiry and cancellation retain the existing guards. A queue receipt means
**queued**, not saved bytes.

## Remaining download scopes

This extension does not cover connector-only cloud references, mounted/shared
library references, alternate preview targets, extra context scopes or image
pointers containing parameters. Unknown attribution structures do not receive
a download handle; the file remains visible and its original conversation is
still available.

The shared asset's `AX` builds a distinct
`/api/library/files/{id}/download` binary route for `sharedLibraryFileId` or
`libraryDownloadId`. It cannot be treated as the JSON signed-URL resolver.
`dEt` also forwards parsed image-pointer parameters and normalizes `#` in a file
ID, but the required parameter meanings and scope combinations are not yet
confirmed. Neither route is claimed implemented by this copy extension.
The subsequent [shared-library attachment candidate](chatgpt-private-shared-library-download.md)
implements a bounded binary-to-native-storage lane for recognized indexed
references. Its native build and live transfer remain pending; standalone and
mounted library, metadata-only references and parameterized pointers remain gaps.

## Verification and grouped acceptance

The new 35-case connector-copy suite first reproduced missing download handles.
The final combined run passed 111 Node runner cases, with no failures or skips:
connector, ordinary and image downloads; conversation file index; private
history projection/transport; image gallery; attachment composer and production
asset-bundle parse. HTTP and native download receipts are synthetic.

Cases cover ordinary/image/project/library combinations, invalid attribution,
separate reference types, immutable selection, authorization failure without
external fetch, late identity/document/route changes and cancellation. No
Kotlin/Gradle build, APK publication or phone transfer occurred in this batch.

Grouped acceptance must select a synthetic connector copy already imported
into a ChatGPT conversation, invoke Download from the production native file
detail, retain private-route provenance and verify the saved bytes. Include an
image or project/library-linked copy when available. Preserve the draft,
selected conversation and voice state; do not claim a speed or heat improvement
without an actual sample, or repeat source implementation as protocol research.
