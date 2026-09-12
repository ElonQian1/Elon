# Canvas Edit Observer

## Scope And Status

- `capability_id=android_chatgpt_private_canvas_original_edit_v1` (existing).
- `code_status=implemented`; `verification_status=offline_verified` for this bridge.
- `completed=false`; original Canvas save/restore/first-share still require a
  disposable fixture in the production native UI. Do not repeat accepted audio,
  captions, dictation, normal sending, or shared-Canvas source viewing.
- Source batch only: no new APK built or installed here. Normal 1681/adapter 361
  remains the last verified native runtime-send release. The new bridge uses
  bindings v17 and edit-context v2, with one new observer asset.

This replaces the invalid `canvasEdits.getState()` admission documented in the
[runtime repair](chatgpt-runtime-bindings-20260912.md). The website does not export
that store. Existing private read/write endpoints, native editor, confirmation,
version comparison, readback and uncertain-write guards are reused unchanged.

## Reviewed Website Contract

Current public generation `web_20260912` only:

- Conversation `Dvt` initializes the Canvas store. `Avt` is the actual dirty
  hook used by the editor, comparing triggered/flushed timestamps for one ID.
  It is a hook, not a standalone getter or a complete save-status signal.
- Editor `ac476d6c-foq8estmw3bbr7op.js` uses this hook and TanStack mutation key
  `['canvas', 'textdoc', 'persist']`. Variables carry `textdocId` and `lastVersion`;
  successful data is the new persisted version. Debounce completion alone does
  not prove persistence succeeded.
- React `2340486e-dyt4epctwx2pn2sj.js` exports singleton factories `zn` (React),
  `Wt` (ReactDOM), and `Ut` (ReactDOM/client). Only these semantic exports are
  admitted; older/unreviewed profiles cannot load them through this path.
- The hook retains its ES-module live binding because `Dvt` initializes it lazily.
  No guessed alias, mutated React dispatcher, synthetic hook runner in production,
  closure debugger, or second copy of the website store is used.

Public asset SHA-256:

| Asset | SHA-256 |
|---|---|
| Editor | `f341cbd1ebc80c14838f6bada429d1826a5a78577f094f5be452ca7663c3c895` |
| React bundle | `bd1f145733f12933c92dd18fbb8e982601c65ef22a41dd2f898fc8f357857261` |

Other module hashes remain in the runtime-repair report. Public downloads are
parsed as source, never executed by research tooling. AST evidence distinguishes
top-level exports from imports and nested variables with the same short names.

## Native Save Admission

1. Read the exact dirty hook in a detached real React root rendering `null`.
   Accept only the committed layout-effect result; synchronously unmount after
   each check. Nothing is added to the visible DOM, no editor is opened, and no
   idle render polling or persistent React subscription is introduced.
2. Reject pending/failed saves for the selected document from the existing query
   client's mutation cache. Unrelated documents and non-Canvas mutations do not
   block it. Missing/unknown protocol state fails closed.
3. Retain bounded failure metadata through mutation-cache GC, without retaining
   content or credentials. Only a later successful persisted version acknowledges
   a failure. One event subscription is reused per identity/document/client;
   document exit or owner replacement disposes it. BFCache preserves its state.
4. Recheck before preflight, dispatch and reconciliation. Native persistence still
   uses one private POST, canonical GET readback and exact-query invalidation.
   Neither a timeout nor an unknown result retries a write through another path.

This observes available official state, not historical state absent before the
observer attached. Device acceptance must include website-save failure/recovery
and late attachment after a previously opened editor; do not infer these from
the dirty hook alone or claim the entire private queue is exported.

## Verification

- `canvas-observer-contract-check-20260912-081111-974`: **142/142** passed,
  zero failure/skip. Covers original document, history, first-share, semantic
  actions, runtime bindings and public-source contract checks.
- Real React 19.1.1/ReactDOM with jsdom 26.1.0 additionally verified subscription,
  committed dirty state, no visible DOM and zero hook subscribers after unmount.
  This is not the phone's canary React runtime or a successful server write.
- Native Canvas smoke, share/content/publish UI contracts and share-read smoke
  contract passed. Android compilation/device-write acceptance are deferred to
  the grouped candidate, not represented as passed by these source guards.
- Wireless ADB answered on the Xiaomi. Production fixture smoke stopped at its
  foreground guard because a separate quant task owned the screen: **zero**
  messages, saves, restores or new links. No Cookie/data/proxy changes.

Next: grouped candidate, one disposable original Canvas through native UI,
save/readback, history cancel/restore, sharing confirmation/cancel, and restoration
of the original conversation. First-publication success remains separate from
merely showing or cancelling its confirmation. Preserve the observed failure
metadata tests when validating the actual website runtime.
