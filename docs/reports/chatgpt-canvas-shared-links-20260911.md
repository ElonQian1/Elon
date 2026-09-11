# Canvas Shared Link Management

## Scope

Capability `android_chatgpt_private_canvas_shared_links_v1` extends the existing
private shared-link owner and production native Share menu. It lists existing
Canvas public links, copies the selected official URL, and cancels that link
only after native confirmation. It does not publish a Canvas, edit its content,
delete the original document, cancel all links, or implement post/workspace shares.

## Current Protocol Evidence

The current official settings module imports the public manager asset
`04d63960-glv7a24oalwx9dld.js`. Its SHA-256 is
`6f9d75e18590007911fe4315d3772e9a2dcab6f77ac1cd402c10e4b3ac9cc722`.
Its `G`, `J` and `se` paths establish these separate Canvas contracts:

- GET `/backend-api/shared_textdocs`, collection `shared_textdocs`.
- Rows use `shared_textdoc_id`, nullable `conversation_id`, and `created_at`.
- DELETE `/backend-api/textdoc/shared/{shared_textdoc_id}` cancels one link.
- The original Canvas is not the DELETE target.

URL helper `9085d576-bg9mgqcle0bn8yw4.js`, SHA-256
`677eb111fd016229009433ae9d00c9d79a844b563332f604533bc2b29edb6eb8`,
builds `https://chatgpt.com/canvas/shared/{id}`. These assets were fetched without
credentials and retained under the local research artifact root. Neither
ordinary conversation share URLs nor guessed UUID-only Canvas IDs are used.

## Implementation

- Shared-link module v4 and adapter 355 reuse the existing authenticated,
  same-origin JSON transport and write owner. No composer or DOM menu is needed.
- Separate conversation/Canvas caches retain the account and document binding,
  60-second read TTL and 120-second selection lifetime. No new polling loop.
- At most 1,000 fetched rows, 100 per native page. Paging is local over the
  returned collection; no unobserved server cursor endpoint is invented.
- Resource-specific selection tickets prevent cross-kind cancellation. A write
  consumes its selection before dispatch. Timeout permits readback, never an
  automatic second DELETE. Only a complete fresh list proves cancellation.
- Canvas rows may lack a source conversation. Receipts contain only safe IDs,
  canonical source paths, dates and pagination metadata, not document names,
  contents or headers. The native list uses cached source titles when available,
  otherwise numbered Canvas labels; exact official document-name parity is not
  claimed. Canvas receipts are bounded to 32,000 characters; ordinary receipts
  retain their previous 18,000-character limit.
- The existing Share menu gains one Canvas management entry. Copy, confirmation,
  read-again recovery and cancel/epoch guards reuse the existing coordinator.
  Semantic list ID: `web-chat-canvas-share-links-list`.
- This is Android's on-device private integration. The web mirror still opens
  the official provider page; no fake browser-side private controller was added.

## Verification And Delivery

Four focused Node suites passed: 181 tests, including cache/resource isolation,
ownership drift, malformed data, maximum-size paging, confirmation and uncertain
write recovery. Both ordinary and Canvas production UI source contracts pass.
Release production and test sources compiled; five focused Android suites passed
20/20 tests, including the actual consumer-to-MCP dispatch and command receipt
boundary. The first compile found a nullable branch expression, corrected before
the successful run. The initial Gradle wrapper returned zero despite compiler
errors; verification therefore checks terminal Gradle output and test-result XML,
not only the outer process exit code. No APK was packaged for this source batch.

`code_status=implemented`; `completed=false`. Authenticated Canvas list/Copy/
cancellation and rendered UI acceptance remain pending, using a disposable link
owned by the test. Do not revoke an existing personal link merely for testing.
This source batch follows the grouped-build workflow; it is not a device pass.
