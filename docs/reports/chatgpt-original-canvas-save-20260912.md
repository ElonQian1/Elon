# Original Canvas Read and Save Core

- capability_id: `android_chatgpt_private_canvas_original_edit_v1`
- code_status: `protocol_core_implemented`
- verification_status: `offline_verified`
- completed: `false`
- Production editor/command wiring and device acceptance: pending.
- No APK release, personal document write, Cookie reset or voice/proxy change.

This extends the [observed editor contract](chatgpt-canvas-edit-publish-contract-20260912.md).
It is not another shared snapshot viewer. Reuse these modules when connecting
the original native editor; do not reimplement its transport or rediscover APIs.

## Ownership and Requests

`chatgpt_web_private_canvas_documents.js` owns account/document/path-bound
selection tickets, a bounded one-minute read cache and a single in-flight job.
Original persisted documents come from GET
`/backend-api/conversation/{conversation_id}/textdocs`; a failed read is not an
empty list. Reads need authenticated identity, not composer DOM or editor imports.
Both `/c/{id}` and `/g/{project}/c/{id}` are owner paths, not request endpoints.

Explicit save sends one POST `/backend-api/textdoc/{textdoc_id}` with
`{version, content, comments}`. It requires the selected original ID, unchanged
fresh version/title/type/content/comments, and a clean website edit queue.
Unchanged content makes no POST. Readback must match the returned new version,
body and comments before reporting `canvas_saved`.

The job has an 18-second total request budget; each request is at most seven
seconds. Selection is consumed immediately before write. A timeout or mismatched
readback leaves an unknown-write guard; it does not fall back or resubmit.
GET verification can confirm the first write. A mismatch requires explicit
acknowledgment of the freshly displayed server version, without another POST.
Temporarily switching conversations cannot discard that guard.

## Website State

The original editor asset `ac476d6c-n5q4k6410vjb4971.js` owns `Da` (save), `Aa`
(queue) and `Pa` (debounce); its exact hash and call sites are in the prior report.

Two additional exports are now mapped only for the observed `web_20260911_b`
profile in runtime bindings v15:

Public source SHA-256: conversation module
`a89952420338983e104e94be5fae8e9ae7a4169b9ea8a4ed30e639641e14f456`;
shared module
`41e6e38589707e7c6ff5d181afa2193dda17f605dfe256224af09e6c85cfa096`.

- `conversation-small-ft205i7yqa6zc2nj.js`: `HH as one` is the existing Canvas
  edit store. `VH` resets both timestamps to `null`; the dirty predicate compares
  null-coalesced trigger/flush values. No React hooks or synthetic optimistic
  entries are used. Existing pending edits are never deleted or acknowledged.
- `4813494d-c6b4nsqqwi13e6rd.js`: `Jh as Z0` is a session-scoped query-client
  getter (`bn` -> `Sn` -> session context), not `useQueryClient`. The composer
  asset imports it as `dp`, passes `dp(sessionContext)` to its query provider;
  the root asset imports it as `vi` and uses it for ordinary invalidations.

`chatgpt_web_private_canvas_edit_context.js` validates the same runtime profile
before write. After confirmed save it invalidates only the exact
`[conversationId, "textdocs"]` query, asynchronously. Native completion does not
wait for refetch. If a website draft starts meanwhile, reconciliation is deferred
without erasing that draft. No page reload, DOM scrape or second query client.

## Comments and Native Editor Contract

`chatgpt_web_private_canvas_document_policy.js` decodes server Unicode-codepoint
ranges to Android UTF-16 offsets; saves encode against the *edited* text.
Comment IDs/text are preserved. Duplicate, missing, invalid, surrogate-splitting
or collapsed previously nonempty anchors reject the write. Removing/editing
comments needs its own explicit decision and is not silently bundled into save.

The pending native editor must track anchor positions through text edits,
retain its draft after failure, show server/native conflict comparison and send
explicit saves through a dedicated `canvas_document` command. It must not reuse
the shared-publication command or truncate original source in a diagnostic receipt.
Expose body/list in a request-bound typed event; diagnostics contain counts only.
No editor should be advertised as delivered until that production wiring exists.

## Verification and Remaining Work

Five Node suites passed 145 checks (zero failure, skip or cancellation), covering
read/cache without DOM/runtime, exact original requests,
selection expiry and identity changes, version conflicts, website dirty/pending
states including real null timestamps, profile drift, readback uncertainty,
single flight, no replay, explicit conflict acknowledgment and Unicode comments.
The existing runtime-binding suite also verifies that old profiles cannot expose
these new aliases. All fixtures are synthetic, no private documents or credentials.

Still pending: production native editor/command wiring, comment anchor tracking
and conflict UI, grouped Android compilation/device save acceptance, original
first-publication, rich editing and revision restore. Do not mark the overall
Canvas capability complete from this protocol-only batch.
