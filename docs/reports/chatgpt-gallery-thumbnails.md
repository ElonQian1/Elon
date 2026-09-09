# Gallery thumbnails and on-demand full previews

## Evidence and scope

This closes the code gap identified by the 1618 cold-page test in
[catalog URL reuse](chatgpt-gallery-catalog-source.md). Reuse its official asset
evidence for `encodings.thumbnail.path`; do not guess URL transforms or another
private endpoint. The existing v6 URL allowlist, native cache, full image
exporter, cursor subpages and explicit official Images entry remain.

## Implementation

- Gallery v8 registers separate opaque thumbnail and full-preview handles when
  a valid, different official thumbnail URL exists. The normal page request
  downloads only grid images. Missing/invalid thumbnail metadata retains the
  previous full-image grid path; already cached full previews are reused.
- Optional `previewHandles` pairs are ordered, bounded and validated natively.
  Older snapshots default to the original `handles`, preserving compatibility.
- Selecting a tile uses the existing `request_image_asset` command. The gallery
  claims only full handles from its current catalog and emits scoped native
  image receipts. No new PageAdapter entry or duplicate image exporter exists.
- The native tile shows a loading indicator, then opens the existing full
  viewer. A failed full read clears the indicator and permits retry. Small
  thumbnails never overwrite the full-preview cache or masquerade as a full
  image. Known small URLs are not sent to Android as private signed URLs.
- The catalog job and the open gallery's preview ownership have separate
  lifetimes. A completed/partial/timed-out catalog does not disable full-image
  viewing. Close, navigation, account/document changes and disposal revoke old
  ownership; late results cannot open a viewer in a different context.
- Full-image URL expiry gets at most the existing one private re-resolution.
  Intermediate failure is suppressed until that bounded operation concludes.
  Duplicate full requests coalesce; closing cancels this gallery's listeners.
- When the grid and selected preview share a full-image handle, they join the
  entire URL-resolution/download operation in either start order. They cannot
  race over the one expiry recovery or publish its first failed attempt. Ending
  the catalog also drops transfer leases, so a later selection cannot join a
  cancelled grid read. Explicit preview ownership remains separate.

## Offline verification

`gallery-transfer-final-20260910-021701-448`: 52 Node tests passed, zero failures,
skips or cancellations. New cases compose the real gallery and image exporter:
no full-image request until click, both cache variants, invalid/missing thumbnail
metadata, partial grid failure, expired full URL, coalescing and owner changes.
The catalog deadline case verifies a selected preview remains usable until close.
The new simultaneous grid/preview expiry test first reproduced a false preview
failure on v7, then passed for both request orders on v8 with exactly one
resolution, one replacement download and one ready event.

`gallery-thumbnail-jvm-20260910-015833-790`: the actual protocol source and its
JUnit class compiled with the repository's Kotlin 1.9.22 toolchain; all 7 tests
passed. This is a focused JVM protocol run, not Android UI instrumentation.
The source-size guard passed nine files. The existing external semantic UI
runner now allows the bounded on-demand preview and reports its loading state.

The first grouped v7 release, `1.1.1619` / source `faedfc7dd`, built and published
successfully. Its SHA-256 is
`10b0cbc6d815f476d0d5f99445c6e27cf248d1f68954472cae160ad6f81bbd43`.
Automatic installation failed because the wireless phone was offline; this was
not a compile or publish failure.

The v8 correction is published as `1.1.1620` / source `91df1aa7a`. Normal Release
build and lint completed; remote APK hash and size matched. Evidence stem:
`gallery-transfer-release-20260910-021941-176`. APK SHA-256:
`1a374d725089e4f794b598a1c9f3d6065005a4d6be8f7b46940323e46326b9e3`.
The packaged gallery asset matches the tested source (SHA-256
`3ec8da4a36443382f2fddc386d623967ed246d45f6bf065c1238cabb00a902db`).

The final ADB inventory has no connected devices. Installation was explicitly
deferred through a process-local disabled target configuration, not the user's
global ADB settings. No repeat bootstrap, microphone, upload, conversation write,
Cookie clearing or proxy modification occurred. The last installed/accepted
version remains 1618; neither 1619 nor 1620 has device evidence. Resume with the
published 1620 artifact and one uncached native gallery page plus on-demand full
preview and return, preserving the previous conversation/draft. Do not rebuild
or repeat protocol discovery just because the handset reconnects. Cold-page
completion, latency and heat improvements remain unverified.
