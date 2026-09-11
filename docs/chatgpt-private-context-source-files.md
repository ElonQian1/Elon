---
version_status: current
reviewed_at: 2026-09-11
---

# Context-source file inventory

## Delivery Boundary

This extends `android_chatgpt_private_file_citation_download_v1`, not a second
attachment system. Context-source policy/owner v1, citation v6, projection v10,
private transport v28, download owner v28 and Android adapter 344 are implemented.
The source path is wired into the existing native Conversation files command.
Synthetic protocol/integration verification passed; grouped Android compilation,
installation and production source-shape/download acceptance remain pending.
Do not mark this scope completed or repeat accepted personal/project TXT tests.

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

Run one grouped Release and the production file sheet against an actual context
source response. Confirm the final source mask, matching rows, one selected
download and actual saved bytes, then restore the original conversation/draft.
The USB device list is currently empty; do not declare device success or
reinterpret an absent device as an application protocol failure.

Accepted scopes remain in [file citations](chatgpt-private-file-citations.md).
Remaining work stays in [the batch map](web-ai-private-native-remaining-batch.md).
