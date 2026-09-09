# Gallery thumbnails and on-demand full previews

## Evidence and scope

This closes the code gap identified by the 1618 cold-page test in
[catalog URL reuse](chatgpt-gallery-catalog-source.md). Reuse its official asset
evidence for `encodings.thumbnail.path`; do not guess URL transforms or another
private endpoint. The existing v6 URL allowlist, native cache, full image
exporter, cursor subpages and explicit official Images entry remain.

## Implementation

- Gallery v7 registers separate opaque thumbnail and full-preview handles when
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

## Offline verification

`gallery-thumbnail-final-20260910-015911-858`: 51 Node tests passed, zero failures,
skips or cancellations. New cases compose the real gallery and image exporter:
no full-image request until click, both cache variants, invalid/missing thumbnail
metadata, partial grid failure, expired full URL, coalescing and owner changes.
The catalog deadline case verifies a selected preview remains usable until close.

`gallery-thumbnail-jvm-20260910-015833-790`: the actual protocol source and its
JUnit class compiled with the repository's Kotlin 1.9.22 toolchain; all 7 tests
passed. This is a focused JVM protocol run, not Android UI instrumentation.
The source-size guard passed nine files. The existing external semantic UI
runner now allows the bounded on-demand preview and reports its loading state.

Release and real-device thumbnail/full-preview acceptance are pending. Do not
claim cold-page completion, reduced latency or reduced heat from offline tests.
