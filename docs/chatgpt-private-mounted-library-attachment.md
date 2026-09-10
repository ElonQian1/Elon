# Mounted library attachment

Capability: `android_chatgpt_private_mounted_library_attachment_v1`.
Code: implemented. Verification: offline verified; grouped APK and production
selection/send acceptance are pending. This is not a `completed` capability.
Source `8ed89f7e4`, adapter 314, mounted attachment module 1, library owner 2,
attachment sender 21. The existing ordinary library attachment remains reused.

## Official contract

The retained public `8b34dbc2-nhot65scqrg20d6p.js` composer has SHA-256
`36644eb82aac9c399bce384c18140f8c878dd780c8f787440b80f27971729733`.
Its `onAttachMountedLibraryFile` callback invokes `SB.attachMountedLibraryFile`:
the prepared branch calls `bS` and retains the original mounted ID/provider/MIME
alongside the returned ordinary file ID, name, MIME, size and optional preview.
`bS` imports conversation export `LTt`, which exports `qR`.

The retained `conversation-small-owrec55n6vm0ekcc.js` has SHA-256
`7973d518b083f0f3e23905a279ed019378481bdbdd10fc0196afe9fc7b3b7d35`.
`qR` POSTs `/files/library/mounted/materialize` with `file_id`, `name`,
`mime_type`, and `index_for_retrieval`, defaulting the latter to **true**.
The download caller explicitly sets false; its success is not attachment proof.
These hashes/caller-export links were rechecked on 2026-09-10. Public source is
protocol evidence, not proof of a successful live-account mutation.

## Production path

- Existing native Library rows expose the same opaque `canAttach` action for
  concrete Drive, Box, Dropbox and SharePoint file IDs accepted by the existing
  catalogue validator. Container IDs, external-account objects, project/scoped
  records, artifacts, saved entities and unsupported media remain rejected.
- An explicit selection enters the existing sender's exclusion boundary and
  verifies ordinary, non-temporary, library-enabled, empty composer context.
  Materialization uses the existing bounded page-local JSON transport, not a
  second uploader, Android HTTP client or independent message POST.
- The selected handle is consumed before the one POST. Duplicate command IDs
  reuse their receipt; timeout, cancellation or invalid output does not retry,
  switch to DOM or publish an optimistic attachment. A new attempt requires a
  fresh catalogue selection. Requests time out at 12 seconds even if a fetch
  implementation ignores abort; response buffering is bounded to 64 KiB.
- Only confirmed metadata enters the existing ready-file store, native card,
  removal and prepared-send lease. The File contains no downloaded bytes. The
  backing ID is distinct from the mounted provenance ID; a stale copied
  `file_id` in the catalogue cannot redirect the materialization request.
- Identity/document/route/model/store/selection changes prevent late association.
  Drafts and messages are not changed by selecting a file. A second context
  validation also checks a PDF type first discovered in the prepared response.
- Known sizes above 8 MiB are rejected. Missing remote sizes remain zero reference
  metadata, matching the official ready shape; this is not a measured byte size.
  Ordinary documents, JPEG/PNG/WebP and the three existing Drive export categories
  are supported. Live-reference/suggestion policy, external-account variants,
  mixed/multiple selections, project and temporary attachment are separate gaps.
  No account policy or speculative live-reference flag is manufactured.

## Verification and next acceptance

`mounted-attachment-red-20260910-130141-305` reproduced the missing mounted
attachment action before implementation. Final related run
`mounted-attachment-final-20260910-130731-030` passed **196/196**, zero failures,
skips or cancellations. This includes the real catalogue/opaque action, bounded
JSON transport, all five concrete ID forms, returned metadata/image preview,
existing native projection/removal/send lease, identity changes and ignored
cancellation. All 109 production JS assets and their assembled bundle parsed.

Related old lease tests still asserted the pre-`d9c615783` cleanup contract.
Loading baseline `20ecd7dd4` composer/sender independently reproduced 6 failures
out of 25 (`attachment-lease-baseline-check-20260910-130638-592`). Test-only
`eb3e2e792` now checks idempotent ACK cleanup of the captured store while retaining
pre-dispatch invalidation and protecting the replacement editor. The production
cleanup implementation was not changed to satisfy those stale assertions.

Next: one grouped normal APK with the
[mounted catalogue download](chatgpt-private-library-mounted-download.md).
Use an existing authorized test cloud file, click the production Library file
menu, verify one named removable card and unchanged draft, then explicitly send
once in an isolated ordinary chat. Verify source/backing association and an
actual file-content response; restore the former chat. No cloud-account linking,
private file creation/deletion or live success is implied by the offline tests.
