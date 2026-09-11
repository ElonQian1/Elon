# Versioned website runtime bindings

## Status

Version 13 adds the phone-observed September 11 build using 58 inspected export
contracts, retaining all earlier profiles. The live `runtime_not_observed`
fallback and 536 passing focused/adjacent checks are recorded in
[the compatibility report](reports/chatgpt-runtime-bindings-20260911.md).
Delivery and live acceptance remain separate from source compatibility.

Version 12 adds only the inspected September 10-b `conversationStore` export
`GJ` (`hS`) for bounded, request-owned regeneration observation. Older profiles
do not guess this mapping. Source/getter/subscription evidence and the 314-case
related run are in [the regeneration report](reports/chatgpt-regeneration-store-20260911.md).
Normal 1644 / adapter 327 is published and installed; the ordinary authenticated
native retry passed with the corrected contract. Other consumers retain their
own acceptance boundaries.

Version 11 adds the second September 10 build observed on normal APK 1639.
It retains all earlier profiles and maps 57 consumed contracts from inspected
public sources. The private model diagnostic had reported `runtime_not_observed`;
132 focused and 304 adjacent tests passed at that checkpoint. Normal 1640 /
adapter 325 was published and installed; subsequent native model admission
and ordinary retry are covered by the 1644 evidence above. See
[source evidence and boundary](reports/chatgpt-runtime-bindings-20260910-b.md).

Version 10 adds exact September 10 local-upload quota/type helpers for
[reverse-order Library/local attachment append](chatgpt-private-library-local-append.md).
Older profiles do not guess these bindings. Their existing consumers are
unchanged; new mixed-order device acceptance is pending.

Version 8 adds the observed September 10 build while retaining all four earlier
profiles. See [current source evidence and verification boundary](reports/chatgpt-runtime-bindings-20260910.md)
and the [September 9 acceptance](reports/chatgpt-runtime-bindings-20260909.md).
It repairs build admission, not proof that every consuming feature has passed
new device acceptance.

Capability: `android_chatgpt_private_runtime_bindings_v1`.
Code: implemented and shipped in `1.1.1547`. Offline contracts and Release
compilation passed. Personal share/create/list/revoke and disposable conversation
deletion passed through production handlers on the current website build. Other
consumers still require their own acceptance; this is not completion of every
private feature or an independent native HTTP text sender. See the
[1547 evidence and remaining regressions](reports/chatgpt-runtime-release-1547.md).

Source-only extension after 1547: the
[additional model catalog](chatgpt-private-model-catalog.md) adds four exact
export mappings for category construction, group IDs, eligibility and its gate.
Its native menu and selection have offline evidence, not device acceptance.

Version 3 adds exact shared exports `Ur` (registered action configuration) and
`zr` (invoke registered action) for both known builds. Public old symbols are
`d2`/`Vzt`; current symbols are `p2`/`iBt`. The new-chat consumer can call the
official `newChat` action without finding a visible button or synthesizing a
router reset. `Ur` reports missing/disabled actions; `zr` selects the last live
registration and enforces the current disabled-group/override state. Its Boolean
return acknowledges invocation, not conversation completion. It must not be
treated as a generic native/MCP arbitrary-action executor. These source bindings
do not establish a real-device new-conversation acceptance result.

## Confirmed regression

Version 5 adds the previously missed composer tool consumer. Old `Ng`/`Bg`
(`JL`/`BL`) map to current `Yg`/`n_` (`OR`/`yR`), not the reused aliases that now
focus the editor or initialize a module. It also identifies the exact compiled
menu owner (`Whn` / `Kgn`) for eligibility capture. See
[tool-state evidence and scope](chatgpt-private-composer-tools.md). Existing
text, model, voice, sharing and attachment export mappings are unchanged.

The installed APK 1545 public-script inventory observed
`c2675c8c-kconnwitb9zzv81k.js`. Its public static module imports identify the
September 7 shared, conversation and composer bundles below. Existing private
consumers still pinned the prior bundles. Export names were also reassigned:
old shared `H3` is the authentication getter, while new shared `H3` is an unrelated
initializer. Updating URLs without remapping exports would call the wrong code.

The new resolver retains both exact known builds. The old bundle URLs are
compatibility keys at consumer boundaries, not requests to import an old runtime
into the new page. A facade maps only verified contract exports; it never passes
unknown or unmapped exports through under legacy names.

## Public evidence

All files were retrieved without credentials from
`https://chatgpt.com/cdn/assets/`. No private request proof, cookies, headers or
conversation contents are fixtures or source evidence.

| Role | September 7 file | SHA-256 |
|---|---|---|
| Anchor | `c2675c8c-kconnwitb9zzv81k.js` | `74cd84f25020a5e3e7e42578ba2329e95ef3cdfda1ff3ae6d035efba66067dff` |
| Shared | `4813494d-o593jrji51wy4azk.js` | `48563cd22f0dafe6c0b89220348fa3add81ff3abb82a62ed9d68a04d569cc375` |
| Conversation | `conversation-small-owrec55n6vm0ekcc.js` | `7973d518b083f0f3e23905a279ed019378481bdbdd10fc0196afe9fc7b3b7d35` |
| Composer | `8b34dbc2-nhot65scqrg20d6p.js` | `36644eb82aac9c399bce384c18140f8c878dd780c8f787440b80f27971729733` |

Compared prior files: shared `4813494d-hrplraurzfyvxb10.js`, conversation
`conversation-small-hiw4wce20lu6te81.js`, composer
`8b34dbc2-kjj15hg4y6iyx13p.js`. React module
`2340486e-dyt4epctwx2pn2sj.js` is unchanged.

AST comparison ignoring local renaming yielded unique candidates for 37 of the
38 consumed exports. Function bodies, object member names and literal protocol
contracts were inspected. The ambiguous async-status store (`Fx`) was resolved
using the official stop function's actual import: old `Fx`, new `Lx`. The
request-active getter is old `Fl`, new `Il`; the official stop function is old
`FVt`, new `hHt`. Normalized AST similarity alone is not permission to map a
future unknown build.

The temporary-chat owner changed from `AKt` to `vqt`; both inspected owners use
the same 30-slot memo contract. The precise current callback still performs
attachment reset, personalization/tool cleanup and router navigation. Its
captured closure is invoked only after committed-owner, current-context and
privacy checks. No component execution, reconstructed closure or DOM click is
used by this path.

## Runtime contract

- Only the official origin and exact known observed assets qualify. The observed
  anchor is evidence for its inspected static dependencies when Resource Timing
  has evicted older entries. Arbitrary prefixes and user-supplied URLs never load.
- Shared, conversation and composer roles must belong to one known build.
  Mixed builds or a build change inside the same document are rejected.
- Consumers reuse page-owned module singleton identities. Imports are coalesced
  per module and cached for the document token; account state remains live in
  the existing official stores and consumer-specific identity guards.
- Import timeout is 1.5 seconds, followed by a bounded 10-second failure cooldown.
  Late results from another document cannot replace current cache entries.
  Clearing Resource Timing alone does not invalidate a successfully loaded cache.
- Missing runtime is unknown, not evidence that the provider lacks a feature.
  Existing explicit official recovery remains available. An uncertain write is
  never replayed through another transport.

## Integration and verification

Consumers: personal sharing, deletion mode, attachment reservations and reuse,
project branch association, model/effort state, direct runtime text submission,
regeneration, stop-generation and temporary-chat transactions. Their existing
scope, confirmation, single-writer and completion checks are unchanged.

Bindings v15 adds only observed September 11b original Canvas edit-store and
session query-client aliases. The [read/save core](reports/chatgpt-original-canvas-save-20260912.md)
checks pending edits and reconciles the exact document query without a reload.
Its production editor wiring/device acceptance remain pending.

The production script catalog was extracted without changing its order;
404 related Node runner cases passed before behavior changes. The runtime batch
then exercised 558 cases. One test still expected the previous regeneration
module version; its assertion was updated and that suite rerun. The initial
integration-stop fixture omitted terminal state, and version admission in the
attachment owner needed the new reservation/library versions; both were fixed
before delivery, without skipping the affected cases.

New integration cases execute real sharing, model and stop owners with the new
export identities. Temporary-chat tests exercise the new callback for both new
and saved chats, and text submission confirms zero extra composer imports.
Resolver tests cover old/new builds, wrong aliases, every mapped identity,
truncated timing, mixed/unknown builds, coalescing, timeout, cooldown, document
replacement, foreign origin and production assembly order.

The installed 1547 production-handler sharing/deletion cases are completed, not
inferred from these synthetic tests. Direct sending was not accepted: the reply
arrived through the existing `template_unavailable` fallback and the native list
retained a thinking-status bubble. Google remains deferred until the remaining
ChatGPT acceptance gate passes.
