# September 15 Runtime Compatibility Investigation

Status: implementation and offline verification completed for bindings 29 /
adapter 407. `web_20260915` is admitted through its exact observed asset set.
Grouped release/device verification follows; no new device pass is claimed here.
Reuse [accepted Writing Blocks 1723](../chatgpt-writing-blocks-native.md),
ordinary text, Search and Image modules.

## Implemented Follow-Up

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
  not a Writing Block. First send with absent composer and independent
  attachment dispatch also need device evidence on the new grouped package.

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

Next: publish one grouped APK, verify profile selection, then resume native
first-send and one-send attachment acceptance. Use a genuine owned Canvas to
verify edit/save/readback; do not count Writing Blocks as Canvas. The mappings
above are implemented and must not be re-researched without regression evidence.
Keep the broad Goal active and distinguish compatibility from live acceptance.
