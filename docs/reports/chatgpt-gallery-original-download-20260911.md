# Native Gallery Original Download

Capability: `android_chatgpt_private_gallery_original_download_v1`.
Status: implemented; device acceptance pending.

## Scope

- Production social AI sidebar -> Images -> image preview -> Download original.
- Reuses the existing versioned private image-pointer download contract,
  native download lease, progress/cancel UI and storage. No DOM click or new
  endpoint, no Android TTS/other system AI substitution.
- Catalog registration is read-only. It does not fetch original bytes until
  the user clicks download. The gallery's bounded JPEG cache remains a preview,
  never an original-file source.
- The page-local handle binds the selected catalog item to its source
  conversation, account, document and live gallery owner. Closing the gallery,
  changing its page or switching identities invalidates selection. Duplicate
  clicks are blocked while a download is active.
- Only ordinary concrete image pointers with source conversation IDs qualify.
  Project/shared/mounted/watermarked or otherwise scoped variants are not
  inferred from ordinary images. Their existing official access is retained.

## Verification

- New regression first failed because gallery registration did not exist.
- Node targeted suite: 75 passed. Covers registration without I/O, scoped
  download query, account/document/owner invalidation and old gallery protocol.
- Android release compile and four targeted unit-test classes passed.
- Production acceptance entry:
  `scripts/smoke-chatgpt-web-gallery-original-download.ps1`.
  Clicks the native UI semantically, verifies exactly one PNG on the handset
  (signature, IEND, dimensions, decoder), checks command receipt and restores
  chat/draft/awake state. Does not export private image bytes.

## Other Pending Fixture

Mounted-library paths already shipped in 1639; not reimplemented here.
Read-only inventory on 1651 found 21 initial rows (20 files, one directory),
with another page available. A fixed-fixture search found 13 rows, including
four owned TXT/PDF/PNG fixtures. None proved mounted-provider scope.
Non-mutable files alone are not mounted evidence. Real mounted acceptance
remains deferred until a corresponding accessible fixture exists; no new
external account, connector or cloud file was created.
