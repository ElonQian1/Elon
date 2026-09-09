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

Android build and phone acceptance of this batch are pending. No measured
latency, cold-page completion or thermal improvement is claimed yet.
