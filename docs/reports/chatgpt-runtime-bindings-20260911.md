# September 11 runtime compatibility

Capability: `android_chatgpt_private_runtime_bindings_v1`, resolver 13,
adapter 333. This repairs existing consumers after an observed website build
change. It does not introduce a second sender or claim every private feature.

## Current device evidence

Xiaomi on normal 1649 / adapter 332 remained connected through wireless ADB,
authenticated, on production native `social_ai` / `chatgpt_web`, with no draft,
stream or dictation. No login, Cookie, proxy or audio state was changed.

A single fixed-text attachment was uploaded and read successfully. The actual
send receipt was the official fallback with `runtime_not_observed`, not
`official_runtime_v1:accepted`. The native original two-message conversation
and empty draft were restored. Log:
`citation-download-1649-20260911-072430-491` (33.5s).
That harness initially stopped on its stricter runtime-send prerequisite; this
is not evidence of a failed upload or download. It performed zero downloads.

A fresh page-acknowledged `runtime_assets` diagnostic identified the four new
public files below. None is an admitted resolver-12 profile. This establishes
the pre-invocation compatibility gap without guessing a network failure.

## Inspected public contracts

The phone-observed files were retrieved without credentials from
`https://chatgpt.com/cdn/assets/`. Acorn parsed the anchor's static imports and
confirmed the three roles plus unchanged `2340486e-dyt4epctwx2pn2sj.js`.

| Role | Filename | SHA-256 |
|---|---|---|
| Anchor | `c2675c8c-bqh1u7ph1mhnj4w5.js` | `34234d33f5602b03f484e2ffcaeffd9055628e3b2893f4e77c5f51ab3e413cb2` |
| Shared | `4813494d-c7ndis1pzhdkm9ow.js` | `c7d29791641a75932761b785e57bf077d57aa0af9bc478c89756ff158f2cbbf8` |
| Conversation | `conversation-small-iklux3elvv7sfvxg.js` | `6b7d52909c6ddde0f9c455a261dc2d32bef08bbeeaeb012d9b36ef176701fda3` |
| Composer | `8b34dbc2-tgz90yfx7ipn1n2e.js` | `3701097324ddee9fbab7c0f9046c4bdba815bed28f3848aa2a91671e080310e4` |

The retained September 10-b AST comparison tool was reused, preserving literal
values, operators and member/property keys while normalizing local identifiers.
Of 58 consumed exports, 56 have unique declaration matches. The batch helper
has two matching declarations but only one exported candidate (`ag`, `V0`).
The seven matching async stores were disambiguated from actual stop function
`dfn`, exported as `BHt`: it reads `Sn` (shared `GS`), active request `Dbe`
(`td`) and state enum `Ur` (`Net`). No similar-looking store was substituted.
Composer attachment validator/store `Ph` is unchanged structurally, including
all methods; tool owner `oyn` and temporary owner `mYt` each uniquely match
their preceding owners. The exact temporary action is recorded independently
in `scripts/fixtures/chatgpt-runtime-bindings-sep11.js`.

The versioned profile retains all six earlier profiles, exact build selection,
account/document ownership, singleton reuse, import timeout/cooldown, and
uncertain-write no-replay. No wildcard imports or guessed exports are allowed.

## Verification

- Before implementation: 8/36 failures in the new-profile fixture run,
  `runtime-sep11-red-20260911-073804-297`.
- After implementation: 116 binding/citation tests passed, zero failures,
  `runtime-sep11-green-20260911-073907-322`.
- Adjacent consumers: 420 tests passed, zero failures/skips/cancellations,
  `runtime-sep11-consumers-20260911-074043-203`. These cover real source owners
  for submit, attachment dispatch, stop, regeneration, tools and temporary mode
  against controlled fixtures; they are not live provider requests.

Grouped normal release and production acceptance are pending below. No thermal
improvement or private-independent Android HTTP sender is claimed.

## Citation acceptance boundary

The additional six cached synthetic candidates (offset 6) were read once on
1649; all six file indexes succeeded and original state was restored. None had
assistant file references. Log:
`citation-fixture-discovery-1649-20260911-071822-644`. Combined with the prior
six candidates, this is fixture discovery, not twelve download passes.

The one new fixture above was not resent. Title-based recovery did not identify
its exact original prompt in the selected cached candidates, so it stopped
without a download; the original conversation/awake state was restored each
time. The test now checkpoints its own new conversation locally and validates
the fixed prompt before resuming, instead of guessing identity from titles.

`smoke-chatgpt-web-citation-download.ps1` uses the actual native header/settings/
file-index/row/Download controls, requires an assistant reference to the fixed
fixture, checks the fresh download receipt and one new saved file, then verifies
78 bytes and its known SHA-256 on the handset. Upload uses the existing fixed
fixture and consumer send once; private-upload and runtime-send outcomes stay
separate from download evidence. Unknown results do not cause a second send.
This harness is not itself a successful citation download. Mounted/PCA/URL-only
cloud references remain separate unresolved scopes.
