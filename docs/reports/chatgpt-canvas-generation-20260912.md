# Original Canvas AI Editing

Status: implemented candidate, offline verified; production-phone acceptance
pending. This is an **official runtime command**, not an independent Android
HTTP generation POST. The parent
`android_chatgpt_private_canvas_original_edit_v1` remains partial.

## Real Provider Contract

Reviewed public assets from the 2026-09-12 profile:

| Module | SHA-256 |
|---|---|
| `d3304073-k8khdx5ezvb9oyu8.js` | `4c59db7c2db5afb92e5b037b891fe28a99bc8ab185170c737aad9587a7d5b559` |
| `4813494d-gf2h57w5fiay19bd.js` | `6c015001732054f4143ef1922609407c540967762109dcd128bbf56706889c3e` |
| `2340486e-dyt4epctwx2pn2sj.js` | `bd1f145733f12933c92dd18fbb8e982601c65ef22a41dd2f898fc8f357857261` |
| `dced7e81-j6uhc8d9bjowrh43.js` | `6818768ef0e29f9778542059c55e68379e668e0907b8bdc7db0920e90766d19d` |

The editor imports `a` (`Hn`) and initializer `i` (`Wn`) from the first module.
The hook builds the actual hidden Canvas context, preserves conversation mode,
and calls composer export `Am` (`IV`) using the Canvas request-completion
callsite. It includes textdoc ID/type/version/content length, `ask_chatgpt`,
`open_in_canvas_view`, selection metadata and a targeted reply. The local action
enum confirms `EDIT=edit`; this is distinct from `ASK`, comment dismissal and
`ACCEPT_COMMENT`. The exact context-building helpers are private to the module,
so the integration invokes the real hook instead of copying its prompts.

The reviewed shared export `MP` lists existing conversation objects; it does not
create another conversation. `tP` is the send-block hook. React exports `In/An`
initialize/provide Intl; `An` stays a live binding in resolver v18. The callback
is captured in a real, detached React root under Intl at dispatch time, then the
root is immediately unmounted. No extra visible page, persistent React tree,
fake hook dispatcher, composer text mutation or composer-ready gate is added.
Permission and conversation mode are read after asynchronous original readback.

## User Flow And Safety

- Native original editor exposes `web-chat-canvas-editor-generate`.
- A nonempty UTF-16 selection opens selection editing; a cursor opens whole
  document editing. Surrogate splits, invalid ranges and overlong instructions
  are rejected. The same original-version/ticket/scope is checked before sending.
- Unsaved local changes must be saved first. AI editing never quietly discards
  or submits a different native draft. The old body stays visible while waiting.
- The prompt and explicit confirmation use the existing `canvas_document`
  canonical action (`generate`), not an unrelated generic chat sender.
- `Hn` does not return the underlying request-completion promise. Therefore a
  successful hook invocation is only `canvas_generation_dispatched`, **not**
  proof of server acceptance or updated content.
- A consumed invocation is never replayed. An uncertain generation blocks new
  Canvas writes and competing native text/regeneration until the exact Canvas
  prompt/version/selection and a terminal assistant reply are observed in the
  original conversation tree. Idle DOM or manual confirmation alone cannot
  acknowledge an unobserved generation.
- Canonical original GET confirms the new version. An observed terminal turn
  with no new version is `canvas_generation_no_change`, not generated content.
  A later completed turn permits ordinary text without requiring the editor to
  stay open; original editing still requires version reconciliation.
- Native waiting checks cached stream state. At most six idle canonical checks
  run within 90 seconds; closing the editor cancels them. No repeated generation
  calls or DOM polling is involved. New content only replaces an unchanged draft.

Semantic controls: `web-chat-canvas-generation-prompt`,
`web-chat-canvas-generation-confirm`, `web-chat-canvas-generation-cancel`.
The existing editor version-check and original-document list remain reusable.

## Evidence

- `canvas-generation-final-contracts-20260912-101801-954`: **543 Node tests
  passed**, zero failures/skips. This includes original save/rename/comments,
  history/export/share, runtime binding profiles, normal sends and regeneration.
- New tests cover whole/selection dispatch, UTF-16, scope/version/identity
  changes, permission changes during preparation, denied/uncommitted hooks,
  one invocation, uncertain-result protection, read-only confirmation and no-edit
  completion. A real React/ReactDOM/jsdom check verifies provider context,
  callback capture, zero remaining subscriptions and no visible children.
- Public assets are hash checked and AST inspected, not executed in Node tests.
  The real-React test uses a synthetic hook; it is not a live website call.
- Regression test assembly now loads the existing production delta-document
  dependency before the stream policy. The missing test dependency was not a
  new production parser regression.
- `canvas-generation-release-check-20260912-101710-319`: Android Release Kotlin
  compilation and **23 unit tests passed** (generation 3, draft 13, original
  document protocol 7), zero failures/errors/skips. The logged Gradle check
  completed in 296.8 seconds without a timeout or stall.
- No new APK was assembled, published or installed for this code batch.

## Remaining Work

Accept this candidate on one disposable original Canvas through the production
editor: whole-document rewrite, selected rewrite, original-context identity,
streaming result, unchanged-body waiting, repeated taps, and version adoption.
Small-screen/landscape layout and real reply metadata still require device
evidence. Existing audio/subtitles/dictation/read-aloud need no repeated research.

Original comment acceptance is not dismissal. The subsequent
[comment acceptance candidate](chatgpt-canvas-comment-accept-20260912.md) now
implements its two-step flow and partial-success recovery; live acceptance is pending.
Faithful Markdown/code export is now an implemented
[text-export candidate](chatgpt-canvas-text-export-20260912.md), pending phone acceptance.
Follow the [remaining map](../web-ai-private-native-remaining-batch.md); do not
start Google before the remaining ChatGPT acceptance gate.
