---
capability_id: android_chatgpt_native_image_asset_gallery_v1
implementation_status: completed
production_default: true
official_page_authoritative: true
---

# ChatGPT native image assets and gallery

The completed capability below records the earlier DOM-discovery baseline.
The 2026-09-07 [private catalog extension](chatgpt-private-image-gallery.md)
reuses this native UI and cache, replacing its transient sync WebView with the
existing identity session and cursor pagination. That extension is source-only
pending grouped APK/device acceptance; the earlier device results below do not
verify the new private transport.

The production friend-chat surface can render ChatGPT image content and open a native
cache-first gallery without navigating the active conversation away from its official
page. The official ChatGPT WebView remains the identity, generation, byte-fetch, and
conversation authority. Android does not copy cookies or request headers into a separate
HTTP client.

Adapter `208` recognizes only allowlisted HTTPS or same-origin blob image content. Page
code derives an opaque `image_[a-f0-9]{16}` handle, fetches and downscales the image inside
the official origin, and sends a bounded JPEG preview plus dimensions through the
origin-checked bridge. Events never contain source URLs, labels, prompts, conversation
text, headers, cookies, credentials, or request bodies.

Android validates the handle, JPEG framing, dimensions, payload size, and protocol
version before saving. The cache is limited to 80 files, 64 MiB total, and 1.1 MiB per
file. Message previews and the three-column gallery use sampled background decoding;
the gallery shares two worker threads instead of creating one thread per image. The
full-screen viewer loads only after the user selects an item.

The gallery renders cached entries immediately. A transient, visually suppressed,
same-profile WebView visits `https://chatgpt.com/images` only when the successful-sync
marker is older than six hours or the user explicitly presses refresh. Each pass imports
at most 24 missing handles, skips cached handles, times out after 35 seconds, and destroys
the transient WebView on completion or failure. A temporary empty DOM remains loading;
it is never interpreted as proof that the official capability is absent. The official
`/images` page remains the complete fallback.

## Verification

- Node contracts cover image-origin filtering, bounded downscaling, opaque handles,
  initially empty asynchronous gallery hydration, cached-handle suppression, terminal
  success, and terminal failure.
- Kotlin tests cover protocol validation, cache freshness expiry, marker isolation from
  image handles, feature routing, status presentation, and rich-content policy.
- Release builds completed with source-size and document-modularity guards enabled.
- Xiaomi device acceptance on `v1.1.1375 (1396)`, adapter `208`, imported 24 images on
  the first pass and 15 additional missing images on the next stale pass, for 39 bounded
  cache entries. Native full-screen viewing and return to the same production chat both
  passed. A fresh reopen displayed all 39 entries immediately without another sync
  WebView. No application data, cookies, or login state were cleared.
- During device acceptance, sampled thumbnails and bounded worker concurrency reduced
  observed gallery total PSS from the earlier 430-517 MiB range to about 356 MiB; native
  heap fell from about 195 MiB to about 103 MiB. Closing the gallery reduced total PSS
  further to about 274 MiB. These are device observations, not a cross-device benchmark.

This capability is complete and production-default. Reuse it rather than adding another
image fetcher or gallery unless current regression evidence invalidates this contract.
