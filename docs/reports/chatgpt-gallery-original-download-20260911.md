# Native Gallery Original Download

Capability: `android_chatgpt_private_gallery_original_download_v1`.
Status: completed; default-enabled; device-verified for ordinary original PNGs.

## Scope

- Production social AI sidebar -> Images -> image preview -> Download original.
- Reuses the server-returned original URL, versioned private content/download contract,
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

## First Device Failure and Correction

Normal 1652 (adapter 335, source `555a2d121`) passed build and `adb install -r`.
The native gallery/viewer loaded, but original authorization returned HTTP 404 at
`/backend-api/files/download/{id}`. The failed command was reported as failed,
with no saved-file claim; conversation/draft/awake restoration passed.

The catalog's `url` is a resource URL, distinct from its asset pointer and
`encodings.thumbnail.path`. The public current conversation module
`conversation-small-iklux3elvv7sfvxg.js` exports `_qr` as `i5`: it fetches the
provided image URL as a blob for download. Current composer
`8b34dbc2-tgz90yfx7ipn1n2e.js` imports it as `J7e`; `SY` chooses the displayed
resource URL, respecting watermarked variants. This is URL-download evidence,
not a claim that every gallery/account download handler was reproduced.

Adapter 336/download module 25 now snapshots a validated catalog original URL
and transfers its unmodified bytes through the existing native byte owner (or
signed-URL system download). It never uses thumbnail/JPEG-cache bytes. If no
URL is supplied, the existing pointer contract remains. An expired URL reports
failure once; it does not silently probe another permission path. Arbitrary
origins and special scopes remain rejected. Device acceptance follows below.

A local same-version research APK was built but not installed: another task
published and installed normal 1653 meanwhile, so Android correctly rejected
the older candidate. No downgrade or data reset was attempted. Final delivery
must include the 1653 mainline changes and keep the research flag disabled.

## Normal 1654 Acceptance

- Published and installed normal `1.1.1654` with `adb install -r` on the Xiaomi
  over wireless ADB; source `4db5261b3cce7a12f666c60b938cbb39b780f243` includes
  the 1653 mainline changes. Research remains disabled.
- APK SHA-256:
  `8e7977219803ce27c723453b9d4ecd3c804b5dedc7173e77ebcef06624b39556`.
- Targeted URL/download regression suite: 131 passed. Normal Release/lint passed;
  the earlier 17 Android unit tests covered the unchanged native integration.
- The native new-chat screen -> Images -> viewer -> Download original workflow
  passed. One download command received `download_saved`; the content request
  returned HTTP 200 at `/backend-api/estuary/content`.
- Exactly one new PNG was saved on the handset: 662,362 bytes, 1254 x 1254,
  valid PNG signature/IEND and successful bitmap decoding. No image bytes were
  exported. The file was retained for the user.
- Conversation/draft and awake settings were restored. No messages were sent,
  generated or deleted. No microphone, Cookie or application-data changes.
- Evidence: `gallery-original-1654-ui-ready-20260911-095433-057`, passed in 33.6s.
  An initial preflight incorrectly required a selected historical conversation;
  the script now also accepts a blank native chat. This was a test-only fix.
- Project/shared/mounted/watermarked sources, other formats and large transfers
  remain separate scopes. Reuse this completed ordinary-image capability;
  do not repeat its protocol discovery without new regression evidence.
