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

## 1617 production acceptance

Gallery v5 shipped in normal APK `1.1.1617` (adapter 312), source `2d1551ba0`.
SHA-256: `2330bef09648fa0bf00ac84957d175f106f6d297b459eecef1f2de993ae636ef`.
Release/install log: `gallery-subpage-resume-release-20260910-005501-651`.
Server verification and unattended replacement installation both passed.

- The native Images entry now produced a catalog and visible image tiles,
  instead of the prior entire-page failure. The first request reached its
  35-second total deadline with a partial page (`gallery_cancelled`); keep this
  cold-load completion/performance issue open.
- The first tile opened the native full-screen image viewer, and closing it
  restored the same gallery page. It was not an official WebView image page.
- Reopening reused downloaded images and completed page 1 with
  `private_image_gallery_ready`. Page 2 then also completed with that receipt.
  Returning to page 1 was already ready at the first UI observation. The
  viewport exposed 15 image tiles, not a claim that the whole page has 15 items.
- Closing the gallery, sending the app Home, and bringing the existing native
  chat forward exercised host pause/resume. Opening Images again completed
  successfully; no `gallery_invalid_request` recurred.
- No second module implementation or DOM gallery was used. The focused test
  batch has 40 passes, zero failures/skips/cancellations.
- Final bounded readback confirmed the original project homepage, zero messages,
  empty draft/pending attachments, ready composer and closed sidebar/gallery.
  The first immediate restore sample was stale; no second navigation was needed.

Native semantic-action logs include `gallery-ui-open-1617-20260910-010340-139`,
`gallery-ui-ready-1617-20260910-010346-084` (bounded observation still loading),
and `gallery-resume-ui-1617-20260910-010927-676`. Subsequent structured UI/MCP
readbacks confirmed the terminal states above. One warm-open semantic click
failed before entering the gallery; a later settled-drawer action succeeded.
Do not count an action runner's zero exit as a completed image sync.

## Remaining work

Catalog parsing, native subpages, preview opening, previous-page cache and
reopening after host pause have real 1617 acceptance and should be reused.
Cold thumbnail completion remains partial: investigate preview scheduling and
the inspected official thumbnail representation instead of increasing retries
or lifting bounds blindly. No thermal or latency improvement is claimed.
Unknown pointer scopes remain partial, not fabricated images. No image
generation, upload, deletion, voice, login or Cookie clearing was performed.
