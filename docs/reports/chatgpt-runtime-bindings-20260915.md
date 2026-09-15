# September 15 Runtime Compatibility Investigation

Status: bindings 30 / adapter 421 published and installed as 1.1.1759.
[Second-rollout compatibility](chatgpt-runtime-bindings-20260915-b.md) and one
native independent existing-project Send passed on 1753; 1754 publishes the
narrow default, with ready/idle read-only admission and successful restoration.
First-send/follow-up and independent local attachment delivery passed on Xiaomi.
The verified attachment scope is default-enabled, reusing the corrected model argument and pre-write
handoff below. `web_20260915` uses its exact observed asset set.
Reuse [accepted Writing Blocks 1723](../chatgpt-writing-blocks-native.md),
ordinary text, Search and Image modules.

## Implemented Follow-Up

- [Voice captions after search](chatgpt-voice-search-captions-20260915.md): current
  native preview gap confirmed; channel-identifier/cache-limit mismatch corrected,
  with 34 passing Android tests; installed in normal 1761 and later 1762.
  Production readiness was verified by MCP; exact live caption acceptance remains
  pending user participation, not installation or an ongoing ADB outage.

- [Current Writing projection and SPA recovery](chatgpt-spa-recovery-writing-20260915.md)
  closes a current-profile read omission and the reproduced stale startup-route
  pointer after process recreation. 1755 route recovery passed; 1756 adds one
  asynchronous startup history read. Adapter 420 preserves a complete private
  history through sparse DOM snapshots; a device read retained all 18 messages.
  Adapter 421's once-per-document rearm is published with 35 passing Android
  tests. Restart acceptance stopped safely because realtime voice is connected;
  resume the existing smoke after the call ends, without another build.

- [Project admission follow-up](chatgpt-fresh-project-admission-20260915.md):
  GLOBAL, same-user owner and matching project config are corrected. After the
  second official rollout was mapped, one native independent project send on
  1753 passed with 25 stream events and exact history/project membership.
  Adapter 417 defaults only the verified existing ordinary project/plain-text
  scope, not broad new/locked/tool/attachment extensions. Earlier fallback
  successes remain separately recorded and are never counted as independent.

- [1749 recovery follow-up](chatgpt-fresh-recovery-suspend-20260915.md) cancels
  automatic history reads on background/offline suspension without replaying
  the original write. Native background/resume acceptance passed with one Send,
  one answer, 42 stream events and exact history. Real network-loss/process
  recreation and original Canvas save remain separate pending acceptances.

- Normal release 1748 from `c83f5dddd232de858e0772f57cb709647f85bffb`, SHA-256
  `e8a7ab3fdecc235b7756a01074ab76a6bcee497bbd94a0a300fe21b84f2ce1e1`, passed
  build, remote hash/version verification and unattended Xiaomi replacement.
  `composer-free-411-production-publish-20260915-060340-983` passed in 419.5s.
  Research is disabled; conversation prefetch retains its enabled default.
  Post-install native reopening/read-only verification confirmed adapter 411,
  authenticated/ready, no draft/stream/dictation/armed trial/pending write and
  no app WebView debugging socket. Zero repeat messages and no data/Cookie reset.
  An initial probe before native binding timed out; opening the production chat
  then passed. This was not another send or a Canvas mutation acceptance.
- Adapter 411 closes a native no-composer admission regression: unlabeled
  dictation controls require actual capture; verified private-editor readiness
  no longer becomes cached DOM readiness. `c10f1613d` implements the fix.
  [Composer-unavailable native acceptance](../chatgpt-composer-unavailable-acceptance.md)
  passed on local research 1747 from `4633785ee`: one Send, one independent
  request/answer, 31 stream events, exact history, 77 valid no-composer samples,
  original route/CSS/awake restoration. The existing production default was
  used. No-usable-composer first send is now verified; a cold page with no
  initialized runtime is a different boundary. Normal 1748 includes this fix.
- Targeted fix regression: 57 Node cases, seven PowerShell negative proofs and
  15 Android unit tests passed. Research signed Release build passed in 370.7s.
  The earlier failed UI attempt dispatched nothing; its exact synthetic draft
  was safely removed and its external ledger archived without replay permission.
- Final default release 1747, source `2c1944a0f38cb2ed1c1a4390dd09f33ae9105625`,
  SHA-256 `7661a081b14c3bacb40b1b1df7246c31446d91289907de337861d617727f81db`,
  passed Release build/checks and unattended replacement installation. Read-only
  check `fresh-attachment-default-release-readonly-20260915-045045-654` confirmed
  adapter 410, authenticated/ready native chat, no armed trial, pending write,
  draft or attachment. Zero repeat sends. Cookies/data were not cleared.
- Default-scope regression: 617 Node tests passed, no failures/skips, plus 22
  attachment evidence cases and existing fresh/Canvas acceptance contracts;
  `fresh-attachment-default-final-regression-20260915-044113-091`. Input readiness
  and actual dispatch use the same allowlist and explicit opt-out. Project,
  temporary, tool, library, new-chat, other-MIME and file-only extensions are
  not included in this promotion.

- Release 1746, source `d3ac8a9b6`, SHA-256
  `63228868f9cff05eb85fcc3dc09514f83c66fe157ef8b03a8cf911b0a829c557`,
  built/published/installed without resetting login/data. The prior proven-unsent
  synthetic draft/files were cleaned after replacement; no replay.
- `fresh-attachment-native-1746-20260915-043457-109` passed: one native click,
  one send receipt, private upload, independent fresh HTTP, 52 stream events,
  complete native TXT/PDF/image facts, exact history, cards/draft cleared,
  original view and awake settings restored. 64.788s includes upload and readback;
  it is not a speed A/B. [Accepted scope and default](../chatgpt-fresh-attachment-text-dispatch.md)
  are existing personal chats with local TXT/PDF/PNG and a nonempty prompt.
  New/project/temporary/tool/library/other-MIME/file-only extensions stay gated.
- Adapter 409's model/handoff regression passed 573 Node tests, with existing
  attachment/fresh-text/Canvas acceptance-helper checks. The public source AST
  audit passed all three tests with no skip. This closes the actual serializer
  and no-dispatch fallback defects; do not repeat the successful synthetic send.

- Release 1745 / adapter 408 built and installed. Its one native attachment
  trial was rejected before preparation/POST (`attachments_active`, dispatched
  false, zero stream events). The fallback saw a phantom active writer because
  the completion callback ran before `finally`. Read-only recovery proved the
  exact baseline unchanged, recorded a non-dispatched attempt and prohibited
  replay. The unsent synthetic draft/files were cleaned on replacement 1746.
- Adapter 409 / transaction 29 releases that slot atomically in the single-use
  pre-dispatch fallback claim. It never releases a dispatched/uncertain writer.
  Disabled attachment extensions now decline synchronously, so normal users
  retain the accepted attachment sender without candidate preparation overhead.
- The serializer's third argument was incorrectly a model slug. Exact public
  source traces `zmt` -> `p_r` -> imported shared `im` -> `model.product_features`.
  Attachment module 3 now passes the current conversation's actual model object,
  verifies its ID and retains ready-file/model/selection revalidation. The
  six-argument Library guard is unchanged. New behavior tests and hash-pinned
  AST assertions cover this contract; 1746 supplies the real successful send,
  native reply and history readback for the narrowly promoted scope above.

- Release 1743 (`0b00df6c389d09b1eeb0575b3686d39350623994`) was built, published
  and installed without data reset. First send and follow-up each used fresh
  HTTP, reconciled exact identity/history and rendered native replies. Two
  clicks, zero replay; observed replies 13.463s / 10.236s, not a latency A/B.
- Attachment upload plus native reply worked, but that send used the accepted
  official runtime, not fresh HTTP. The attachment reservation accidentally
  disabled both fresh owned-file requests and the unsafe captured-text template.
  Adapter 408 separates these: fresh requests require the native file lease;
  captured-text fallback stays prohibited. Disappeared files cannot become a
  plain-text send. The extension remains trial-gated pending physical proof.
- Its synthetic answer used `3 solid blue squares` / `1 solid red circle`.
  Evidence now accepts that exact variant, still rejecting wrong/missing counts.
  Read-only recovery confirmed the native response and closed the external
  pending ledger with transport `official_runtime_v1`; no resend, no fresh pass.
  Later owned attempts must retain this exact baseline and use a new prompt.
- Canvas acceptance initially stopped before Send: `semantic_control_missing`.
  A screenshot established that an unrelated release 1744 update sheet was
  covering the native composer. The runner now dismisses only the app's known
  update sheet via "Later"; it does not accept any system authorization. Empty
  attempts sent zero messages; only the exact synthetic draft was removed.
- After dismissing the sheet, one native request completed and the native
  Canvas list opened, but the original textdocs endpoint returned zero documents
  (`canvas-native-dismiss-update-1743-20260915-035458-059`). Explicit Canvas
  selection was unavailable; private eligibility diagnostics confirmed
  `canvas raw=0, menu=0, reason=raw_missing`. This is observed provider/model
  capability data, not a missing DOM element. Save/restore device acceptance is
  deferred until a genuine owned Canvas exists. No original document was edited
  or shared, and the starting conversation was restored.
- Adapter 408 routing regression: 570 passed, zero skips/failures, plus 22
  attachment evidence cases and existing fresh/Canvas acceptance contracts;
  `attachment-routing-final-regression-20260915-035310-069`.

- All 100 consumed exports are resolved, including the 13 previously ambiguous
  wrappers. Hash-pinned public AST checks verify actual exports, import edges,
  account selection, WEB prefix, real React factories and lazy live bindings.
- New history hydration reads pending-message existence and respects the
  existing account/conversation/abort/shouldApplyResponse guards. It never
  resends a missing message. Attachment serialization explicitly omits the
  sixth active-Library-context argument, so an unrelated mounted file cannot
  enter the native ready-file selection. Consumed validation methods match.
- Fresh text, writing context/library, original-image download and writing
  prewarm accept the same exact profile. An operation cannot mix profiles.
- New personal plain-text first sends now default to the committed in-memory
  conversation/controller/draft owner even with no mounted composer element.
  Login, runtime initialization and actual owner state are still required;
  this is composer-DOM independent, not page-bootstrap independent.
- Canvas reuses the existing native editor and single-POST/version-readback
  save loop. Generation and directive-aware Markdown export use reviewed
  September 15 modules. Unknown cold-start versions do not poison the export
  cache; each operation captures its profile and rejects mid-operation changes.
- Canvas's official restore mutation uses `versionInt`, not save's
  `lastVersion`, despite sharing the persist queue. The observer now recognizes
  both proven shapes, retains failures across GC and allows only a later
  confirmed version to clear them. Unknown/mixed shapes remain blocked.
- Targeted batch regression passed (771 cases, 4 optional skips), followed by
  writing/runtime regression. Public-source and Canvas contract checks passed:
  `runtime-canvas-current-contracts-20260915-025816-108`.
- Production Canvas mutation acceptance still needs a genuine owned Canvas,
  not a Writing Block. No-usable-composer first send subsequently passed on
  adapter 411 above; cold-bootstrap and network-loss recovery remain separate.

The sections below preserve the pre-implementation investigation and device
baseline; their candidate lists are historical, not remaining mapping work.

## Previous Device Boundary

Normal APK 1739 / adapter 405 remains installed. Its source is
`80d4674d12913b6b4b2aff29685bf343aa4fa94c`; Image default source `82cb5027c`
is a later grouped-release candidate. The handset is online and unlocked.
No microphone, Cookie, application-data or proxy changes occurred.

- `fresh-attachment-native-1739-20260915-015221-730`: stopped at the tool
  ownership guard, before staging. Zero native send actions.
- `fresh-attachment-native-prepared-1739-20260915-015342-597`: after a current
  tool read (zero inherited tools), the independent trial did not arm. Three
  local synthetic fixtures were staged and removed, but never uploaded/sent.
- Read-only trial result: `control=identity_unavailable`, `armed=false`,
  `pending=false`, `attempts=0`. This code alone does **not** prove lost login:
  `FreshTextContext.stamp()` also returns null for an unsupported runtime or
  unloaded shared module. Do not weaken identity checks to admit it.
- The existing `runtime_assets` probe confirmed the new files below. Each
  probe restored the original project chat. Original drafts and login survive.

## Public Source Evidence

Assets were downloaded without account headers and parsed, never executed.

| Role | Observed File | SHA-256 |
|---|---|---|
| Anchor | `c2675c8c-b7tr2e0z4wqiq8hm.js` | `1b1a03a6f37c5f65f6e2405b77d29f0c394d4d4669cd70dde08ff2752bb59324` |
| Shared | `4813494d-i88ebrgl0r2g94a4.js` | `26a355cdf5b2463576675472901d4ea1d974765971208cfacdfb0aa853c0eddf` |
| Conversation | `conversation-small-c89mq7wpr5yt4chy.js` | `80f338297f77ccfe91b2ee86c22888606da5c662bbb3ad09c81f1867173952c7` |
| Composer | `8b34dbc2-bb1thqn2oci02lnx.js` | `852e5603f1d10542e8752f72d7a864cfa2442d8f3f46a8a3c9069a0c93a006ca` |
| React | `2340486e-lo737pyjfygyimqo.js` | `a0e3af9dc19a43aff7b55fe57a6ebca9f8ce108eb599880091abbec880551e9d` |

The expanded baseline covers **100** production-consumed canonical exports.
87 have a unique normalized-body candidate; 13 require further review. A
normalized body erases local names and cannot prove dependency equivalence.
Neither a candidate nor an exported name is sufficient production admission.

| Contract | Evidence / Remaining Review |
|---|---|
| `shared.M$` | Multiple local wrappers, only `q2` exported; check batching dependency. |
| `shared.Fx` | Stop export `fGt` / local `ZM` calls `wd`, imported from shared as `wC`; resolves the ambiguous status getter. |
| `textLockedChatPin` | Old local `hw` reads `mw`, whose body validates `accountKey` before returning `pin`; trace this edge, not any generic context getter. |
| `textClientConversation` | `NX` / `nX` candidates; verify the `WEB:` prefix dependency. |
| `writingLibraryAccount` | Old local `qC` reads `sw`, initialized from a `.data` account query; generic getter candidates are not equivalent. |
| `textHydrateHistory` | Old `BEn` / `gy` becomes candidate `VOn` / `jy`; now inspects pending drafts and invokes additional handling after network history. Review the new branch before allowing reconciliation. |
| `textSerializeAttachments` | Old `Ypt` / `nhr` becomes candidate `zmt` / `p_r`; now adds a separate mounted-file attachment branch. Review against `FreshTextAttachments`, especially Library scope. |
| `writingLibrarySessions` | Four lazy-wrapper candidates; identify the sessions query through its dependency. |
| Composer `fh` | Candidate `Fh` / `OB` has changed upload/remove/Library store logic; compare the methods actually used by native attachment retirement. |
| React `reactApi` | Existing advisory analyzer counts nested same-name assignments; use top-level lexical definitions, not its 239-definition result. |
| React `reactDom` / `reactRoot` | `Ut` / `Ht` wrapper candidates require factory dependency review. |
| React `intlProvider` | Lazily assigned provider; resolve initializer and live binding, not a guessed value. |

Tool owner `Qbn` matches old `myn`; temporary owner `DXt` matches old `JYt`.
The temporary callback still calls a file-store reset before navigation, so
preserve draft/attachment guards. Matching owner bodies are advisory evidence.

## Reusable Work

- `chatgpt-runtime-bindings-sep12-full.cjs` adds missing independent-send,
  Canvas and React symbols to the existing fixture, without changing old tests.
- `analyze-chatgpt-runtime-contracts.cjs` optionally audits the fourth React
  role when declared by the baseline. Missing/duplicate imports fail closed.
- A behavioral test loads the real repository resolver with fake namespaces
  and checks that this baseline covers every consumed alias, including lazy
  live bindings. No downloaded code or identity data executes in these tests.
- `smoke-chatgpt-fresh-attachment-dispatch.ps1` uses the existing synthetic
  TXT/PDF/PNG batch and production native Send once. It requires fresh private
  upload, independent HTTP receipt, completed matching native answer facts,
  history reconciliation, empty attachment cards and view restoration.
- Old answers, wrong provider/conversation, incomplete native output, duplicate
  prompts or uncertain writes cannot count as passes. Pending writes remain
  in the external ledger and are never replayed automatically.
- Admission now precedes file/draft staging. A second read-only check before
  Send prevents using an expired or changed trial. No fallback send can pass.

## Verification And Next Action

- Runtime comparison plus existing binding behavior: **33 passed**, no skips;
  `runtime-research-and-attachment-contracts-20260915-021348-683`.
- Native attachment reply evidence: **16 passed**;
  `fresh-attachment-evidence-final-20260915-021451-521`.
- Semantic runner Java compilation and D8 conversion passed;
  `attachment-native-runner-compile-20260915-021559-605`. This is the external
  acceptance runner, not a new APK build or actual attachment Send.
- Public AST audit: passed in 31.9 seconds;
  `runtime-sep15-public-contract-audit-20260915-021418-436`.

Public artifacts and comparison/index JSON are retained outside worktrees in
the common Git `ai-research-artifacts/runtime-20260915` directory. The older
baseline is `runtime-20260912-a`; Acorn is under `runtime-tools/node_modules`.
Reproduce with `CHATGPT_AST_PARSER` set, using the existing analyzer's five
arguments: old directory, new directory, full baseline fixture, observed anchor,
output directory. Do not restart browser scraping or resend old fixtures.

Current next: use a genuine owned Canvas to verify edit/save/readback; do not
count Writing Blocks as Canvas. Cold-bootstrap and special abnormal-recovery
device cases remain distinct from the accepted native no-usable-composer
first send. Current mappings and the accepted local attachment
scope are published; do not re-research them or repeat their send without a regression.
Keep the broad Goal active and distinguish compatibility from live acceptance.
