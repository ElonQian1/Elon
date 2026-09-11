# Original Canvas Native Editor

- capability_id: `android_chatgpt_private_canvas_original_edit_v1`
- code_status: `implemented`
- verification_status: `offline_verified`
- completed: `false`
- Production editor/command wiring: implemented; grouped device acceptance pending.
- No APK release, personal document write, Cookie reset or voice/proxy change.

This extends the [observed editor contract](chatgpt-canvas-edit-publish-contract-20260912.md).
It is not another shared snapshot viewer. The production entry is
`Conversation actions -> Canvas`, separate from public sharing. Reuse these
modules rather than rebuilding the transport or rediscovering APIs.

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

Correction from the September 12 public-source audit: bindings v15 added two
names, but only the query-client getter was an actual export. Bindings v16
remove the invalid edit-store mapping. Original Canvas save/history/first-share
remain unaccepted and fail closed at `canvas_runtime_unavailable` until the
real page-owned edit queue can be accessed safely.

Public source SHA-256: conversation module
`a89952420338983e104e94be5fae8e9ae7a4169b9ea8a4ed30e639641e14f456`;
shared module
`41e6e38589707e7c6ff5d181afa2193dda17f605dfe256224af09e6c85cfa096`.

- `conversation-small-ft205i7yqa6zc2nj.js`: `HH as one` is an **import** from
  the shared module, not an exported Canvas edit store. The actual local `HH`
  store has `userEdits` and `timestamps` but is not exported. The September 12
  build similarly keeps its local `dir` store unexported. Shape-only mock tests
  did not establish this bridge; no existing pending edits may be bypassed.
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

`WebChatCanvasDocumentsCoordinator` uses the production consumer port and the
dedicated `chatgpt_canvas_document` / `canvas_document` command. Adapter 359
registers its action owner and request-bound `canvas_documents` display event.
MCP diagnostics contain counts, not the body, IDs, credentials or selection tokens.
An opaque identity/document scope is stable across refreshes but changes when
the account or WebView document changes. Old native drafts cannot acquire a new
account's selection ticket. No composer readiness gate is added to Canvas reads.

`WebChatCanvasEditorView` renders the full inert source in an Android editor.
`WebChatCanvasDraft` follows comment anchors through edits and requires explicit
re-anchoring after a range is deleted or becomes invalid. It preserves comment
IDs/text; it does not silently drop comments. Input beyond 128 Ki UTF-16 units is
rejected as a whole rather than truncated. Source HTML is never executed.

Save failures retain the in-memory draft. Refresh does not adopt a newer base
silently: the comparison displays both full versions, with explicit choices to
keep the local draft, adopt the server version, or continue editing the draft
against the reviewed version. The latter does not POST until Save is pressed.
Unknown writes are checked with GET and require explicit comparison if different.
Draft retention is scoped to the live coordinator, not process-death persistence.

## Verification and Remaining Work

Five Node suites passed 145 checks (zero failure, skip or cancellation), covering
read/cache without DOM/runtime, exact original requests,
selection expiry and identity changes, version conflicts, website dirty/pending
states including real null timestamps, profile drift, readback uncertainty,
single flight, no replay, explicit conflict acknowledgment and Unicode comments.
The existing runtime-binding suite also verifies that old profiles cannot expose
these new aliases. All fixtures are synthetic, no private documents or credentials.

The native-editor batch passed 167 Node checks across six suites (original policy,
documents, canonical actions, shared content, publishing and runtime bindings).
Canonical checks cover production menu/asset wiring, typed output, explicit
confirmation, unknown-write verification and opaque scope changes.

Release compilation and `:app:testReleaseUnitTest` passed, with 28 JUnit cases
and zero failures/errors/skips: CanvasDocuments 5, CanvasContent 6, Draft 7,
OperationReadiness 10. The final run was `canvas-native-editor-release-tests-fixed`
(272.4 seconds, `BUILD SUCCESSFUL`). The first compile caught a missing empty-row
accessibility ID, fixed before this passing build. Programmatic sheet replacement
also uses a generation guard so old dismiss callbacks cannot cancel the new view.
Source-size guard passed 25 files; documentation guard passed two files. The
remaining-work map is near its size limit but shrank in this batch. No APK was
assembled or installed; device pixels and a real save are not yet accepted.

First-publication and history preview/restore are now implemented in the
[management batch](chatgpt-canvas-history-first-share-20260912.md), awaiting grouped
device acceptance with this editor. Rich formatting/comment authoring remain
outside these batches. Do not mark overall Canvas complete from offline tests.
