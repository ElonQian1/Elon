# September 11 second runtime build

Capability: `android_chatgpt_private_runtime_bindings_v1`, resolver 14,
adapter 337. Repair of an observed compatibility regression, not a new sender.
Device verification of the new profile remains pending.

## Evidence and boundaries

- Normal 1654 remained authenticated on the production native chat, connected
  through wireless ADB. The existing synthetic project-media conversation was
  verified by its fixed prompt and uploaded fixture, not its title alone.
- One citation follow-up reached a completed response through the old send
  fallback: `template_unavailable` / `runtime_not_observed`. No retry was sent.
  Original conversation/draft/awake state was restored. Evidence:
  `project-citation-1654-ui-20260911-101121-116`.
- A page-acknowledged `runtime_assets` read observed the new anchor below.
  The bounded inventory was truncated, so role filenames were resolved from
  that anchor's parsed static imports, not inferred from missing inventory rows.
- The four public modules were retrieved without credentials. No login,
  audio, proxy, Cookie or application-data changes were made.

| Role | Filename | SHA-256 |
|---|---|---|
| Anchor | `c2675c8c-m4ftlj32vtu9aroq.js` | `b04a890f2f4075dd5d2c5c8e5d9ddb3b710bd59c107ea423ee1d67edaba92900` |
| Shared | `4813494d-c6b4nsqqwi13e6rd.js` | `41e6e38589707e7c6ff5d181afa2193dda17f605dfe256224af09e6c85cfa096` |
| Conversation | `conversation-small-ft205i7yqa6zc2nj.js` | `a89952420338983e104e94be5fae8e9ae7a4169b9ea8a4ed30e639641e14f456` |
| Composer | `8b34dbc2-fpy4mlfnxc115y6k.js` | `c4b74136b4efd5255fd91e9c0f2f9c382212ffc28f5ecee7aaa564ed05e36cec` |

Of 58 consumed symbols, 55 have unique normalized-declaration matches. The
batch helper has two candidates but only one exported candidate, `U0`. The
actual stop function `pN` (export `GUt`) imports async state `GS`, active request
`td`, enum `Iet` and tree selectors `OJ`, resolving the ambiguous store by its
real dependency edges. The selector object retains 54 methods; 52 match.
The two changed methods are display-item continuation projection and async-task
continuation detection. The object identity is confirmed by the actual stop
import; consumer shape/ownership checks stay in place. Tools `Pvn` and temporary
owner `Kqt` match independently; the exact temporary action is in the fixture.

All seven earlier profiles remain. Unknown or ambiguous builds are not admitted
by prefix. Existing singleton identity, timeout/cooldown, document ownership and
uncertain-write no-replay policies are unchanged.

## Reusable analysis

The earlier external AST comparison helper is now a committed, parameterized
tool: `scripts/analyze-chatgpt-runtime-contracts.cjs`. It parses public JavaScript
with Acorn, resolves anchor imports, compares exported declarations and owners,
and saves hashes plus symbol ranges. It never evaluates downloaded modules,
modifies production mappings or automatically selects ambiguous candidates.
Identifier-normalized similarity is advisory evidence, not semantic proof.

Set `CHATGPT_AST_PARSER` to an installed Acorn module when it is not resolvable
from the current toolchain. Invoke with five arguments: old asset directory,
new asset directory, trusted prior repository fixture, observed anchor filename
and output directory. Keep artifacts outside tracked source; do not include
runtime tokens, request headers or conversation contents.

## Checks

- Before mapping: 11/57 failures, including `runtime_not_observed`.
- After mapping: 108 binding/current-consumer tests passed.
- Adjacent submit/stop/model/tool/temporary/attachment/citation tests: 477 passed.
- Reusable analyzer: 5 tests passed, including preserved literals/property keys,
  ambiguous stores, repeated assignments and refusal of external/traversal imports.
- Updated citation acceptance contract parses and passes. Android release and
  native post-install verification follow separately; source tests are not a
  successful provider request.

## Project citation remains separate

The same project fixture was reopened without sending another message. Native
Conversation files contained five files including one assistant TXT reference.
Its actual Download action failed at `GET /backend-api/files/{id}/simple` with
HTTP 404, before file authorization or saving bytes. Evidence:
`project-citation-existing-1654-ui-20260911-102854-879`; one download attempt,
zero sends, original state restored. This is not a completed project download.

Current public `98ca14f9-e91ouw2dmvxxzxzk.js` still calls shared `uDt` (`$p`)
for metadata before URL resolution. Its query keeps file, conversation and
project scope; locked-chat context additionally uses the official unlocked
state. No evidence justifies dropping scope, bypassing metadata errors or
guessing a second authorization route. The known personal TXT capability is
not broadened by this failed project sample.
