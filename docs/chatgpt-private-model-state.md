# ChatGPT private model selection state

## Status and scope

- Capability: `android_chatgpt_private_model_preset_state_v1`.
- Extensions: `android_chatgpt_private_model_version_state_v1` and
  `android_chatgpt_private_service_tier_state_v1`.
- Status: implemented, source-only candidate; not `completed` or device accepted.
- Contract version: 1. Production native model selection reuses the current
  official picker state and its model/effort mutators when the guards pass.
- Scope: available normal-chat presets and thinking effort, available model
  versions, and the official standard/fast response tier when offered for the
  current selection. Internal/special models retain the existing menu path.
- This is a page-runtime private state bridge, not an independent Android HTTP
  sender. WebView still owns identity and the live official conversation.
- This batch does not replace work-model selection, restricted-model effort
  controls or text/image generation POST. Temporary chat has its separate
  [guarded state transaction](chatgpt-private-temporary-chat.md).
- Google stays last. Existing native official audio, captions, dictation and
  read-aloud are unchanged. No APK was built or installed for this source batch.

## Inspected official source

These exact modules were inspected on 2026-09-07. No endpoint is inferred from a
menu label, and no hook or React render function is invoked outside React.

| Official module | SHA-256 |
|---|---|
| `https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js` | `89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0` |
| `https://chatgpt.com/cdn/assets/conversation-small-hiw4wce20lu6te81.js` | `296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1` |
| `https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js` | `9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5` |

The composer's `zqn` creates the `fqn` picker element even while its dropdown is
closed. The committed trigger ancestors contain `dropdownContent.props` with
the conversation, models, denials and `composerIntelligencePickerState`.
`bucketSelections` already contains the official preset/effort combinations.
The adapter consumes these filtered props instead of copying account policy or
calling the hook-based `Hqn` picker-state builder.

The normal-chat preset callback in `fqn` uses these inspected exports:

| Namespace | Export | Purpose |
|---|---|---|
| Shared | `M$`, `uo`, `t4` | Signal batch, work-mode state, model-lane constants |
| Shared | `RW` | Official default-model/effort preference mutator |
| Conversation | `Nrn`, `yRt`, `Grn` | Current model, effort store, apply model |
| Conversation | `vRt`, `p8t` | Allowed efforts and current model-denial result |
| Conversation | `l0`, `M1t`, `Rdn` | Service-tier store, Pro defaults, picker-surface enum |
| Composer | `Ih` (`mz`) | Official different-model selection action |

Same-model effort changes retain the official preference update; Pro effort
changes retain the official Pro-default mutator. These mutators own their
requests. A successful local readback does not prove that their asynchronous
server preference persistence has completed.

### Version and response-tier extension

The normal-chat Advanced view's `Tqn` filters model versions with `win` (version
categories), `f8t` (availability), `IX` (category lookup), and `Ein` (official
version label). The private view applies the same eligibility checks, including
the restricted picker's catalog-model check. It does not invent version names.

The inspected `Dqn` version action first matches the current bucket/model lane
using `ay`, `iy`, and `ry`. It reads Auto switching through `Hrn` and Pro effort
through `Jrn`, then falls back to an available model in that version if no preset
matches. The adapter mirrors that sequence with the same pure helpers, writes
`Rrn().setConversationVersion`, and calls the existing effort/model transaction.
It does not invoke the hook-based picker or change service tiers during version
selection. It confirms version, model, effort and unchanged tier after writing.

`fqn` passes its service-tier callback to `Gv` (`E5i`). The latter exposes the
normal-chat fast-mode switch only when standard and fast options both exist.
The adapter uses the same `c0` resolver and `l0().setServiceTier`, then checks both
conversation and draft tier. Hidden, upgrade-preview, stale or unrecognized
tier state is not offered as an actionable private choice.

## Ownership and lifecycle

1. Resolve one committed picker ancestor from the connected model trigger.
   Reject disabled, ambiguous or uncommitted pickers and unknown object schemas.
2. Bind exact URL, document token, private account identity and conversation
   object. Match the official server conversation ID to the route, including
   ordinary, temporary and project/new-chat contexts. Reject unsupported routes.
3. Load only the exact inspected modules already observed as resources or
   module preloads. Import once per document, with single-flight, a 1.5-second
   timeout and 10-second failure cooldown. Warm requests do not open a webpage
   menu or start recurring timers.
4. Re-read the selected version, model permissions, allowed effort and current
   model/effort before mutation. Version/tier actions also compare conversation
   version and tier, and confirm the resulting state. Work mode is excluded.
5. Use the existing official stores and actions, then read back the actual model
   and effort in the same conversation. Do not report a local label change as
   a successful selection. Unexpected post-write state fails confirmation.
6. Return fresh opaque option handles. One immediate receipt supports an
   idempotent retry without another mutation. Runtime objects and credentials
   remain in the page closure, never in native menu snapshots.
7. Unknown runtime/schema before writing may use the existing menu. After any
   attempted mutation, never automatically replay through DOM. A changed
   conversation during import cannot fall back into the new conversation.
8. Dismissal cancels pending private reads and invalidates options without
   synthetic touch or keyboard focus. Already-open official menus retain their
   close path. Advanced opens private version/tier choices when recognized;
   Back returns to presets, and Other official models retains the old entry.
   Navigation invalidates the previous selection receipt. Restricted pickers
   without presets do not show a nonexistent Back entry.

## Implementation and validation

- `chatgpt_web_private_model_contract.js`: source-pinned binding, schema,
  eligibility, model/effort/version/tier selection and readback.
- `chatgpt_web_private_model_state.js`: bounded module cache, catalog handles,
  selection receipt, cancellation and lifecycle.
- `chatgpt_web_adapter_composer.js`: production request/select/dismiss wiring.
- `ChatGptWebPageAdapter.kt` and `chatgpt_web_adapter_bootstrap.js`: asset order
  and lifecycle registration only; shared adapter-version edits are untouched.
- `scripts/test-chatgpt-web-private-model-state.js`: synthetic contract and
  production-adapter tests, with no real accounts or network requests.
- Native composer semantics distinguish `model`, `model_version` and
  `service_tier`: only thinking presets form a slider. Version/speed choices
  use their confirmed selected flags; both navigation entries remain visible.
  The popup bounds its height and scrolls longer advanced lists.

The 2026-09-07 focused Node run passed 108 cases covering this capability, tool-state integration, menu
dismissal/submenus, attachment-composer bundle parsing and the send observer.
These are fixture-based tests,
not a live private transaction or an Android compilation result.

The subsequent version/tier batch passed 122 focused Node cases across model
state, temporary state and attachment-composer integration. It covers fresh and
stale bindings, restricted/version fallback, permissions, navigation, duplicate
selection, ignored/partially applied mutations and production command wiring.
Native policy and semantic tests were added but have not been compiled or run in
this source batch; include them in the grouped Android check.

Grouped device acceptance still needs production native UI selection across
model/effort presets, versions and available response tiers, reopening the menu,
switching chats, and one explicit test send to confirm the effective selected
state. Check native scrolling, Back/Other navigation and an unknown runtime
preserve existing functionality. No measured latency, heat or
power improvement is claimed. Do not repeat already accepted unrelated work.
