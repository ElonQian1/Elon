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

The release-time ADB inventory had no connected devices. Installation was explicitly
deferred through a process-local disabled target configuration, not the user's
global ADB settings. No repeat bootstrap, microphone, upload, conversation write,
Cookie clearing or proxy modification occurred. Device acceptance at that point
remained on 1618. The following USB acceptance supersedes that installation gap.

## USB acceptance on 1620

On 2026-09-10, reused the published artifact above with an exact SHA-256 match;
replacement installation and production MCP version readback both passed.
No rebuild was needed. The Xiaomi USB transport was healthy; wireless TCP could
not connect because the phone and PC were on different, non-reachable subnets.

- The native feature sheet contained `图片已更新`. The existing external UI
  runner initially reported `gallery_feature_missing` because it only matched
  `图片`/`图像`. One filtered accessibility inspection confirmed the actual
  consumer control. The runner now admits this observed badge suffix and can
  resume selection from the already-open sheet; no product selector is bypassed.
- The real native Images entry opened the gallery, pages 1 and 2 became ready,
  and pagination reached page 4, not previously opened in the earlier acceptance.
  That page reached the native ready state with 15 visible image tiles, without
  a partial/failure state. This is one successful cold-page sample, not a
  repeated timing benchmark or proof that every page/connection is fast.
- Bounded protocol capture recorded the image catalog GET 200 and ten content
  GET 200 responses. It held 12 records and dropped 15, so those are observed
  records, not total network-request counts. Only field shapes were inspected.
- Selecting image 1 opened the native full viewer. Closing it restored ready
  page 4; closing the gallery returned to native `social_ai` with authenticated,
  ready composer, empty native/official drafts and no streaming/dictation.
  The conversation had two loaded rows at final observation versus zero early
  after installation; exact pre-install conversation identity was not captured,
  so this does not prove same-conversation restoration across installation.
- A fresh capture around the preview click had zero records. The existing
  full-image cache path worked, but distinct thumbnail/full on-demand network
  transfers were not demonstrated. Do not call that narrower scope accepted or
  infer reduced bytes, latency, heat or battery usage from this pass.

Evidence stems: `gallery-1620-usb-install-20260910-032417-760`,
`gallery-open-1620-20260910-032545-952` (initial runner mismatch),
`gallery-select-1620-20260910-033006-803`,
`gallery-page1-wait-1620-20260910-033027-476`,
`gallery-page2-1620-20260910-033110-468`,
`gallery-cold-page4-1620-20260910-033213-923`,
`gallery-cold-page4-wait-1620-20260910-033236-241`, and
`gallery-full-preview-1620-20260910-033337-022`. Subsequent semantic/MCP readbacks
confirmed the two returns and stopped capture. No send, upload, generation,
microphone, Cookie clearing, app-data clearing or proxy change was performed.
Reuse the accepted private gallery/viewer/paging path. Per the latest user
priority, defer distinct-thumbnail efficiency and thermal/battery investigation
until remaining private-API functions are complete; these performance questions
must not hold up the functional batch or cause another gallery-only APK build.
