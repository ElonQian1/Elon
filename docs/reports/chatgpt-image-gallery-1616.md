# Generated gallery: real catalog and resume regressions

## Device evidence

Normal production APK `1.1.1616` (adapter 312), source
`2973e1e1fda9046e02a7a10c2f4672dbe43a77fb`, was published and automatically
installed without clearing data. APK SHA-256:
`1a11aeb710cbcd405805e166ff38bf12622349f2b21f16a760bb6945a24f53a4`.
Build/install log: `gallery-project-grouped-release-20260910-002704-129`.
The authenticated production native ChatGPT drawer and Images entry were used,
not a test page or a direct private command substituted for the native entry.

1614 had returned HTTP 200 for `GET /backend-api/my/recent/image_gen`, with
an object containing an `items` array. Its generic error obscured the cause.
1616 produced the precise receipt
`private_image_gallery_unavailable:catalog:catalog_page_limit` after a fresh
page load. The native screen was failed, not empty and not accepted.
The requested `limit=25` is not an invariant of the server response.

A separate attempt after background/resume returned `gallery_invalid_request`.
The bridge's host pause disposes the gallery. Same-version asset reinjection
then reused the disposed instance. A VM lifecycle regression reproduced this
independently of account/network state.

Some acceptance actions stopped at `foreground_package_mismatch` when another
app held the screen; one action found the already-open dialog rather than its
drawer entry. These are not catalog protocol failures or successful UI checks.
MCP became responsive again after bringing the existing app forward. No app
crash was observed in the current crash-buffer evidence.

## Corrections

Gallery v5 separates bounded server batches from native pages. It accepts at
most 256 items within the existing 512 KiB response limit, caches at most three
server batches for two minutes, and exposes at most 25 items per native page.
Local bookmarks hold server cursor plus offset. The server cursor advances only
after all items in its batch; previous navigation and expired-cache reads keep
the selected offset. Cursor cycles, malformed data and exhausted bounds still
fail explicitly. No item is silently truncated to satisfy the UI page size.

Same-version reinjection replaces an explicitly disposed gallery, but preserves
a healthy instance. Disposed requests cannot emit into the resumed UI. This
change is confined to the gallery owner; the shared adapter entry, voice,
captions, dictation and identity transport are unchanged.

Red evidence: `gallery-resume-red-20260910-004551-013` and
`gallery-subpage-red-test-20260910-005203-804`.
Final focused evidence: `gallery-subpage-final-20260910-005243-835`.
The tests cover 61-item batches, server-cursor continuation, previous pages,
expired subpage offsets, disposal/reinjection and the existing real exporter
composition. Passing tests are not account preview acceptance.

## Remaining acceptance

The v5 source correction still needs a grouped APK installation and native
catalog/preview/pagination/reopen acceptance. Unknown pointer scopes remain
partial, not fabricated images. No image generation, upload, deletion, voice,
login or Cookie clearing was performed for this gallery investigation.
