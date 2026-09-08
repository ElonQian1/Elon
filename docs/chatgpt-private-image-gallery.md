---
capability_id: android_chatgpt_private_generated_image_gallery_v1
implementation_status: implemented
verification_status: offline_pending_device
delivery_status: published
---

# Private generated-image gallery

Latest grouped release: `1.1.1574` (code 1574), source
`8c1b974862e73896903c723eb1d4eaebe9063a54`, includes gallery v2 and the shared
pointer parser v1. Release build and server verification passed. APK SHA-256:
`341cbf0cdb108632ce3a24846bab741977b870bb3ded00544fb5033fd4f3fa8c`.
All 100 packaged adapter assets match source. Evidence stem:
`web-chat-stop-gallery-grouped-release-20260908-182738-650`.
Autodeploy initially stayed disabled while another app held the phone screen.
After the user's renewed request to continue, 1574 was installed with `adb
install -r` and production native chat was ready through MCP. The current page
is a guest session, not an authenticated image library. Gallery device
acceptance is still pending; no account preview, latency or thermal improvement
is claimed, and no login or data clearing was performed.

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
and `download_intent=false`. It requires `status=success` and either a signed
HTTPS `oaiusercontent.com` URL (or subdomain), or one of the two exact same-origin
Estuary content routes described below. Preview bytes use no cross-origin
credentials and reject redirects. The inspected official shared module has SHA-256
`89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0`.

This contract is based on current official source, not a successful current
account API request. Bounded pointer parameters are now implemented below;
shared/library/connector scopes and unrecognized shapes remain partial.

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
316 seconds. All 79 page-adapter assets also parse successfully. That initial
source batch did not package an APK; subsequent delivery is recorded above and
below. Device acceptance remains pending; no live gallery, latency or thermal
success is claimed by these offline tests. Do not mark this capability completed yet.

Next acceptance uses the production social-chat Images action: verify the first
page against the account library, next/back ordering, warm reopen, preview/viewer,
close during a request, same-conversation/draft preservation and explicit official
fallback. Record private-route evidence and only then mark completed. Reuse the
implementation; do not build another gallery or repeat protocol discovery.

## Same-origin preview source follow-up

The grouped release through `1.1.1544` includes the private gallery but its image
exporter v3 only accepts external storage URLs. The official shared source above
also recognizes `/api/estuary/content` and `/backend-api/estuary/content` and
passes the file resolver's returned URL to a download anchor without imposing an
external-host restriction. This establishes the supported source shape, not the
current account's resolver response. A mocked exporter in the catalog tests did
not exercise this incompatibility.

Exporter v4, now included in grouped APK 1545, reuses the released download URL policy in
`chatgpt_web_private_content_source.js`. Only absolute or root-relative ChatGPT
URLs for those two exact paths are accepted. Bytes stay in the page and use
ambient cookies, never copied authorization headers. Redirects and a mismatched
final response URL fail closed. Existing signed external URLs, DOM compatibility,
image MIME checks, 12 MiB limit, scaled JPEG output, cancellation and scope guards
are unchanged. The page adapter loads the policy before both consumers.

The new regression suite composes the real catalog, resolver and image exporter,
including a warm reopen with no network requests. It separately covers source
rejection, cookie isolation, response URL validation, MIME/size limits and account
change during fetch. These are synthetic offline tests, not account-library or
visual acceptance. APK 1545 compiled, published and installed exporter v4. The
global adapter remains 293 because other worktrees own pending version-only
edits; the exporter and APK versions identify this delivery. Actual gallery
preview/viewer acceptance is still pending. Do not mark the capability completed
or claim a device preview fix from source tests alone.

The related Node run passes 96 cases, including seven new private-image content
tests, with zero failures, cancellations or skips. Evidence stem:
`chatgpt-image-content-regression-20260907-190656-837`. The initial pre-fix run
demonstrated rejected same-origin previews, then hit its 120-second timeout due
to a test waiting for a fetch that the old policy never starts. That test now
also observes early completion, so policy rejection fails promptly.

## Shared pointer parser, 2026-09-08

Gallery module v2 reuses the existing conversation-download pointer parser,
now extracted into `chatgpt_web_private_image_pointer.js`. It does not introduce
another resolver, downloader, WebView, polling loop or credential store.
The extraction alone passed 46 focused tests without changing download behavior.

The retained current shared asset `4813494d-o593jrji51wy4azk.js` has SHA-256
`48563cd22f0dafe6c0b89220348fa3add81ff3abb82a62ed9d68a04d569cc375`.
Its `kEt` resolver still splits pointer queries with `URLSearchParams`, takes
the last repeated value and replaces `#` in the file ID with `*`. The current
conversation asset SHA-256 is
`7973d518b083f0f3e23905a279ed019378481bdbdd10fc0196afe9fc7b3b7d35`;
`cNt` retains the existing recent-image route, cursor and 120-second cache.
These public bytes were hash-checked, not an authenticated preview transaction.

The gallery now accepts the same bounded ID/query forms as conversation image
downloads. Immutable selected parameters feed the existing preview resolver;
conversation ID, inline preview and non-download intent cannot be overridden.
The complete pointer participates in cache identity, so variants cannot reuse
each other's thumbnail. Native events still contain only opaque image handles.
Explicit project, post, library or connector metadata is rejected until its
resolver is implemented, rather than silently dropping that scope. A partial
page keeps other successful images. v2 reinjection disposes v1 once and does not
stack requests. Account, document, route and cancellation guards are unchanged.

Four new cases failed against v1, including ignored explicit metadata scope.
The final related run passed 178 Node cases with no skips or cancellations,
including the real exporter, scoped downloads and history projection. The
ordered 100-asset Android script bundle parses successfully. Evidence stem:
`image-gallery-pointer-final-20260908-181510-021`.
HTTP and image bytes in these tests are synthetic. No APK was built or installed
for this source batch. The phone's foreground belonged to a separate grid task
and was left untouched. Grouped acceptance still needs an actual generated
image, plus a parameterized pointer when available, with preview bytes and warm
reopen verified from the production Images UI. Do not mark `completed` yet.

Gallery v3 also consumes the [bounded segmented-pointer extension](chatgpt-private-image-download.md#bounded-segmented-pointers-2026-09-08).
It encodes the full ID through the same preview resolver and preserves distinct
cache handles and no-request warm reopen. Parser 2 and download 13 share the
same implementation; 161 related offline cases pass. This extension is
source-only for grouped delivery, with real preview acceptance still pending.
