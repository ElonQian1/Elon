---
capability_id: android_chatgpt_private_generated_image_gallery_v1
implementation_status: implemented
verification_status: offline_pending_device
delivery_status: source_only
---

# Private generated-image gallery

## Scope

The existing production native Images screen now requests the generated-image
catalog through the active ChatGPT identity WebView. It does not create a second
WebView, navigate the active conversation, scan images from DOM, or scroll the
official Images page. It reuses the existing bounded JPEG exporter, disk cache,
three-column gallery and native full-screen viewer. Google is unchanged.

This is a replacement for catalog discovery, not a reproduced image-generation
transaction. The existing Create Image action and explicit Official Images
entry remain. The older DOM sync module is retained but is not called by the
production gallery controller. Failure does not silently switch transports.

## Inspected official contract

The public `conversation-small-hiw4wce20lu6te81.js` asset inspected on 2026-09-07
has SHA-256
`296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`.
Its `LMt` query reads `/my/recent/image_gen` with `limit: UMt` (`25`) and
`after: pageParam`. It starts without a cursor and uses response `cursor` as
the next page token. Its cache stale time is 120 seconds. `VMt` maps
`asset_pointer`, `conversation_id`, `message_id`, `encodings.thumbnail.path`
and `created_at`; uploaded images and image-style suggestions are separate APIs.

The candidate uses same-origin GET `/backend-api/my/recent/image_gen`, validates
`items` and `cursor`, and keeps cursor ordering rather than scraping rendered
tiles. Ordinary `file-service://` and `sediment://` pointers use the inspected
`Zy` / `WXe` / shared `dEt` preview resolver:
`/backend-api/files/download/{id}`, optional `conversation_id`, `inline=true`
and `download_intent=false`. It requires `status=success` and a signed image
URL on HTTPS `oaiusercontent.com` or a subdomain. Preview bytes use no cross-origin
credentials and reject redirects. The current shared module has SHA-256
`89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0`.

This contract is based on current official source, not a successful current
account API request. Pointer query parameters, shared/library/connector scopes
and unrecognized pointer shapes remain partial, never guessed.

## Ownership and limits

- The page transport owns credentials. Native events contain opaque handles,
  dimensions, bounded JPEG data, request IDs and pagination metadata only.
- Account, document and current URL fence every read and preview result. Closing
  the gallery or disposing the adapter cancels its work. Stale request IDs cannot
  update a reopened dialog. Gallery assets do not enter conversation-image retries.
- The catalog cache holds three 25-item pages for two minutes. Up to 256 cursor
  positions are retained per document/account. Expiry revalidates page payloads
  without losing the cursor; refresh resets the catalog. Account changes clear it.
- Native image cache bounds stay at 80 files / 64 MiB. Reopening a warm page with
  cached previews sends no catalog or download requests. A cold opening does not
  display unrelated cached chat attachments as the generated-image library.
- Reads have six-second deadlines and 512 KiB JSON caps. At most two preview
  exports run concurrently, with the existing eight-second per-image deadline.
  The gallery job stops after 35 seconds; the native readiness wait is bounded.
- A validated terminal empty catalog is empty success. Malformed catalogs are
  failures. Unsupported pointers or failed previews produce a partial page while
  successful images remain visible. No write request or generation is dispatched.

## Verification and acceptance

The focused Node runner passes 18 cases: private catalog/resolver shape, cache
reopen, pagination and expiry, account/document isolation, malformed/empty data,
partial image failures, cancellation, adapter integration, and existing image
export/DOM fallback suites. The older fallback test now waits for its terminal
event instead of asserting after a fixed 80 ms delay under build load.

Android Release production and test-source compilation passed. All 33 tests in
the private-gallery protocol (6), existing WebChat protocol (26) and image-cache
(1) suites passed, with no failure, error or skip. The first compile attempt was
stopped by the wrapper's 180-second silent-output watchdog, not a source error;
the bounded retry allowed 600 seconds of silent compilation and finished in
316 seconds. All 79 page-adapter assets also parse successfully. APK packaging and
device acceptance remain pending; no live gallery, latency or thermal success
is claimed by these offline tests. Do not mark this capability completed yet.

Next acceptance uses the production social-chat Images action: verify the first
page against the account library, next/back ordering, warm reopen, preview/viewer,
close during a request, same-conversation/draft preservation and explicit official
fallback. Record private-route evidence and only then mark completed. Reuse the
implementation; do not build another gallery or repeat protocol discovery.
