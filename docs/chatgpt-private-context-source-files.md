---
version_status: current
reviewed_at: 2026-09-11
---

# Context-source file inventory

## Delivery Boundary

This extends `android_chatgpt_private_file_citation_download_v1`, not a second
attachment system. Context-source policy/owner v2, citation v8, projection v11,
private transport v28, download owner v30 and Android adapter 346 are implemented.
The source path is wired into the existing native Conversation files command.
Synthetic protocol/integration verification and 42 grouped Android unit tests
passed. Release 1662 / adapter 346 is published and installed; native inventory,
refresh recovery and the shared citation-download path were checked on-device.
New context-source shapes and cloud-source opening still lack live samples.
Do not mark this scope completed or repeat accepted TXT tests without a regression.

The new scope covers concrete file identities in completed inline or supplemented
context sources, including PCA files admitted by the returned source mask. It
does not implement arbitrary URL-only cloud downloads, connector authentication,
past-chat navigation or memory actions. Existing file ID, deletion, project,
library and conversation-scope validators still apply. A filtered empty result
is distinct from an incomplete/failed read.

## Protocol Evidence

Public `conversation-small-ft205i7yqa6zc2nj.js`, SHA-256
`a89952420338983e104e94be5fae8e9ae7a4169b9ea8a4ed30e639641e14f456`,
was inspected on 2026-09-11. `l5i` issues same-origin POST
`/backend-api/sidebar/conversation_context_sources` with `conversation_id`,
`message_id`, and `expand_partial_inline`. This is a read operation despite POST;
it is not an independent text sender. `T5i/w5i/C5i` consume status, source and PCA
mask events, and mark completion at clean EOF. `Aun` uses the shared v1 delta
decoder. No live account response for this new source shape has been captured.

`n5i/e5i` extract inline citations, apply string outer-origin overrides, and
replace source identities before filtering/limits. `s5i` selects or combines
inline/supplemental states; `c5i` avoids fetching completed inline metadata and
pending inline finalization, but allows explicit expansion of inline-only data.
`JVi/KVi/x5i` apply mask URL/type selection and newly revealed source ordering.
The shared bundle's `Qh/mX` exports define memory-category and matched-reference
identity semantics; these are source counts/identities, not downloadable files.

The companion `8b34dbc2-fpy4mlfnxc115y6k.js` consumer `AKn` independently
excludes URLs whose outer metadata record has `deleted === true`. This exclusion
survives supplemental UUID replacement and an allowed PCA mask. Policy v2 applies
it after source selection; direct inline parsing uses the same filter before row
limits. A changed inline fingerprint invalidates old cache approval, and replacing
the native inventory revokes its previous download handles. Inner deletion checks
remain unchanged. Exact URL equality and a boolean true match the observed code;
an absent URL or a similarly named file is not evidence of deletion.

The current file helpers are `eia/Yra/ria`; `Zra` reads `cloud_doc_url` or
`extra.cloud_doc_url`. Composer `FKn` carries that as `externalUrl`, even without
a concrete file ID. This is source-link metadata, not a cloud-to-file download
protocol. URL-only sources now enter the native file index as `kind: source`;
concrete downloadable files retain their original row positions and download
flow. Native source opening is implemented but not yet device-accepted.

## Ownership and UI

- Reuse existing page-local identity and bounded HTTP reader. No Cookie/header
  export, DOM traversal, new login path, automatic send or write replay.
- Bind the initial history GET and subsequent sources to the same account,
  document and page generation. Reject late results after navigation or identity
  change; explicit inventory may target another conversation without navigating.
- Reuse the selected visible history branch; do not fetch alternate regenerations,
  analysis or every historical message. Each invocation requests at most eight
  messages, two concurrently, under one four-second source-read budget.
- Cache at most 32 message graphs for two minutes in page memory, keyed by
  conversation/message and inline fingerprint. Identical requests share work.
  Explicit refresh progresses the next uncached batch; no idle polling is added.
- Bound SSE body to 1 MiB, seed to 256 entries/256 Ki characters, and source graph
  to 256 entries. Source parsing opts into strict normal-JSON errors as well as
  the existing strict delta decoder. Truncation/errors cannot approve early PCA
  entries while silently losing a later removal or mask.
- The page-local weak overlay leaves official metadata unchanged. PCA approval
  cannot be forged with a JSON field or copied object; it expires with the owner
  and is revoked by replacement. Policy reinjection preserves one live instance.
- Publish existing/cached file rows first, then the enriched snapshot. The native
  sheet keeps observing until the command settles, not merely its first snapshot.
  Partial failures keep ordinary attachments and show partial/error state instead
  of claiming that the provider has no files. Downloads retain opaque handles
  and existing scoped authorization/native byte saving.

## Verification and Next Step

### Cloud Source Links

Capability `android_chatgpt_private_cloud_source_link_v1`: code implemented,
JavaScript offline verified, native unit execution/build/device acceptance pending.
The entry is enabled in adapter 346; no separate experimental switch is needed.

The existing native file detail dialog shows `Open source` for URL-only cloud
documents and retains `Open conversation`. It reuses `MainExternalActions`,
opening the explicit HTTPS destination only on a user click; no identity headers,
Cookie copying, connector login, automatic network request or file-ID synthesis
is involved. This is a native entry for provider source-link metadata, not a new
private download endpoint or a replacement Android document implementation.

The source parser reads only `cloud_doc_url` / `extra.cloud_doc_url`, not an
arbitrary citation URL. It respects category, deletion, resolved PCA masks and
page-local approval. Native parsing independently rejects executable/local
schemes, URL credentials, control characters, invalid URI escapes and nonstandard
ports. Source rows cannot carry download handles. The click checks the same
consumer and a matching fresh cached row; logout/cleared or replaced lists cannot
launch an old selection. MCP exposes availability, not the destination URL.

`test-chatgpt-web-private-source-links.js` exercises URL-only rows, exact source
fields, unsafe links, PCA ownership/deletion/masks, duplicate URLs, ordinary file
positions and the no-DOM/no-download path. The source producer and Kotlin parser
share `webchat/private-source-links-contract.json`. Added Kotlin parser, link
policy and presentation tests passed in the grouped Android run below; this
does not establish live cloud-source opening or provider source-mask acceptance.
`cloud-source-links-final-js-20260911-164351-509` passes 138 runner cases.
The follow-up `cloud-source-links-wire-verified-20260911-164451-373` passes
27 affected cases, including the real private list-command emission matched to
the shared native protocol fixture. All 113 registered assets and the combined
bundle parse. Tests use synthetic responses, not live cloud connector data.

Reuse lesson: distinguish a provider file ID, a byte-download authorization and
an external document link. A URL-only row must not be dropped merely because it
cannot pass the download validator, nor may it inherit a guessed download ID.
Keep account/selection checks and unchanged attachment positions in the shared
inventory instead of introducing a parallel cloud file store.

### Existing Source Coverage

Source tests: `test-chatgpt-web-private-context-sources-policy.js` and
`test-chatgpt-web-private-context-sources-integration.js`. They cover source/mask
delta channels, inline selection, late replacement/deletion, malformed/oversized
streams, owner changes, cache/single-flight, bounded batches, uncooperative body
deadlines, actual list-command snapshots, existing download integration and asset
reinjection. `context-sources-release-checks-20260911-155807-434` passes 211 runner
cases, zero failures/skips, plus the history script's internal assertions.
All 113 registered adapter assets and their combined bundle parse. An old
reinjection test still expected download/gallery versions 14/8; it now checks
the actual module versions while retaining its disposal/identity assertions.
Native file-save acknowledgements are synthetic; the native polling
check is a source contract, not an Android execution test.

The outer-deletion regression first reproduced three failures on the prior code.
`context-source-deletion-verified-20260911-162601-806` then passed 131 targeted
runner cases with zero failures/skips; all 113 registered assets and the assembled
bundle parse. Coverage includes exact outer flags/URLs, different supplemental
UUIDs, masks, pre-limit filtering, warm-cache invalidation, handle revocation and
upgrading captured policy/download owners before idempotent reinjection.

### Grouped Android And Device Evidence

`private-files-release-unit-20260911-1658` passed all 42 tests in seven suites:
source-link policy, file parsing/MCP/presentation, download dialog and conversation
deletion/lifecycle. There were no failures, errors or skips. Release build and
publication `private-files-release-publish-20260911-1701` passed for `1.1.1662`
(code 1662), source `b205ff59746d7a8905e609f9f548eed2cd3d7c6e`, APK SHA-256
`7dcf8f763cf9fe0fb6eb5cdce8d30abc342a151c5535b51d49db924c50b23cb8`.
The whitelisted Xiaomi received the normal replacement install over wireless
ADB; MCP confirmed 1662 and the native `social_ai` / `chatgpt_web` surface.

`private-files-native-inventory-1662-20260911` exercised three existing test
conversations through the production menu. Two passed; one refresh hit
`files_read_timeout` after 3139 ms, with cached rows retained. The run is recorded
as failed, not silently converted to a pass. A single targeted retry,
`private-files-native-timeout-retry-1662-20260911`, passed: two rows, three refresh
taps coalesced into one 1077 ms read, the same sheet retained and no overlapping
reads. The other successful cases had five rows and a valid empty inventory.
Both runs restored the prior conversation, draft and awake setting; no messages
were sent. This establishes timeout recovery, not the absence of network stalls.

`private-files-native-bytes-1662-20260911` checked the changed download owner's
shared path once using the existing project TXT fixture, without another upload
or send. The native file-detail/download UI saved 78 bytes; on-device SHA-256
matched `75e2ed9bfe5772c9918e552ed07c2c0e689e7039367c81bb6906c63e396fa1f3`.
The private download metadata request returned HTTP 200. Original conversation,
draft and awake state were restored. No Cookie or application data was cleared.

These samples did not exercise a live supplemental/PCA source mask or a URL-only
cloud document. Their source-shape and `Open source` acceptance remains pending;
do not repeat the successful ordinary inventory/download cases to fill that gap.
Next acceptance needs an actual matching source response and native source row,
not a new APK build unless code changes are required.

Accepted scopes remain in [file citations](chatgpt-private-file-citations.md).
Remaining work stays in [the batch map](web-ai-private-native-remaining-batch.md).
