# Generated gallery catalog URL reuse

## Scope and evidence

This follows the real cold-load gap in `chatgpt-image-gallery-1616.md`:
APK 1617 displayed previews but the first page reached the 35-second deadline.
Keep the accepted catalog parser, subpages, native viewer and owner-resume fix.

The public official asset `conversation-small-hg48c5uox88r7a00.js` was inspected
on 2026-09-10. SHA-256:
`7bfb494a2d582faba39c835c20feced3d1c81805835f7e3d97aa9ce0c713060a`.
Its `yPt` catalog mapper retains the catalog `url` and maps
`encodings.thumbnail.path`; `xY` selects `watermarkedUrl` or `url`, and the small
image rail prefers the thumbnail. No guessed private endpoint is introduced.

## Implementation

- Gallery v6 reuses a validated full catalog `item.url`, avoiding a separate
  download-URL resolution request per uncached image when that URL is usable.
- It keeps the existing content endpoint / HTTPS oaiusercontent allowlist,
  account and document ownership, opaque native handle and exporter limits.
  Credentials or signed URLs are not exported into the native UI receipts.
- A failed direct read may use the existing private file resolver once under
  the same deadline. It does not reload the official page or retry writes.
- Full preview quality remains the existing bounded 1024-pixel export. A small
  thumbnail is not silently substituted for the full-screen preview.
- Native pagination becomes available when the active request's catalog has
  supplied its cursor, even if preview downloads continue. A previous request's
  cached snapshot cannot unlock navigation for the new request.

## Verification

Focused Node batch `gallery-catalog-paging-green-20260910-012542-629`:
45 passed, zero failed, skipped or cancelled; source-size guard passed six files.
New real-exporter composition cases cover direct URL reuse, one expired-link
fallback, invalid URL rejection, bounded failure and account changes. A pending
preview supersession case confirms next-page results cannot be overwritten by
the abandoned page. Warm cache reuse remains covered.

## Normal APK and production acceptance

Published and automatically installed `1.1.1618` on the authorized Xiaomi,
source `97241e245`, adapter 312. APK SHA-256:
`5d70b5cad87d69f9f39f7abe667e838ed1b367af7892dcd0705835f50e4998ed`.
Release/build/server verification/install log:
`gallery-catalog-paging-release-20260910-012735-946` (465.8 seconds).

- The native production Images entry completed with
  `private_image_gallery_ready`; cached page 2 was ready at the first observation
  after its page action. No official gallery page was opened.
- Page 3 had not been loaded in the prior acceptance. After its catalog arrived,
  the native semantic state was `loading=true`, `page=3`, and both navigation
  buttons were enabled. Before the current catalog arrived, navigation remained
  disabled and retained the old page label, as intended.
- The bounded protocol capture included the catalog GET 200, a string
  `items[].url` field and ten direct `/backend-api/estuary/content` GET 200
  responses, with no file-download resolver among those captured requests.
  It held 12 records and dropped 11, including unrelated website telemetry;
  this is direct-path evidence, not an exact total request count. Capture was
  stopped afterward, and no signed URL, request credential or image content was
  exported into the evidence.
- Page 3 still hit the 35-second overall deadline and ended partial with
  `gallery_cancelled`. The semantic wait runner itself passed, but the sync did
  not. Cold-page completion therefore remains open.
- A loaded image opened in the native viewer. Closing it returned to the same
  partial page 3; returning to cached page 2 completed with a ready receipt.
- The gallery was closed and the original project homepage restored. Final
  structured state confirmed native `social_ai`, closed sidebar, authenticated
  and ready composer, zero messages, empty draft, no dictation or streaming.
  No upload, send, image generation, microphone, login or data clearing occurred.

UI logs: `gallery-open-1618-20260910-013615-397` and
`gallery-cold-page3-1618-20260910-013745-589`; subsequent bounded semantic/MCP
readbacks confirmed preview, cache return and restoration.

## Remaining optimization

The catalog URL fast path and pagination admission are implemented, published
and exercised; reuse them. They remove a redundant resolver stage but do not
prove lower total latency or heat. The grid still obtains bounded full preview
bytes for every item. Next evaluate distinct official thumbnail sources for the
grid and on-demand full preview loading, without sacrificing viewer quality or
merely extending the deadline. Do not mark gallery cold loading completed.
