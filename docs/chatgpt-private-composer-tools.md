# ChatGPT private composer tool state

## Status and scope

- Capability: `android_chatgpt_private_composer_tools_state_v1`.
- Status: implemented, source-only candidate; not `completed` or device accepted.
- Provider contract version: 2, using the shared versioned runtime bindings.
- Production wiring: native Tools -> existing composer adapter -> official live
  tool signal. Search and Create Image use this path when its guards pass.
- No separate feature UI, account, WebView or parallel tool state store is added.
- Google is deferred. Model, effort, temporary-chat mutation and independent
  image-generation/text POST are **not** implemented by this capability.
- September 8: APK 1559 guest inventory exposes Search, while image creation is
  a login upsell. The previous adapter required an authenticated identity and
  both hints, and missed the shared runtime migration. The current candidate
  fixes those specific gaps; device verification is recorded separately.

## Verified source contract

Inspected official module:
`https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js`

SHA-256:
`9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`.

`Whn` receives the current conversation, composer controller, model and raw
`availableSystemHints`. Its normal tool menu calls `BL` (export `Bg`), which
updates `JL(controller)` (export `Ng`) through `fHt`. The hint values are
`search` and `picture_v2`. `ysn` reads the same `JL` signal into official request
preparation; this is not a local label-only update. Do not invoke hook-based
`QY`/`ZY` or React render functions outside React.

The companion `conversation-small-hiw4wce20lu6te81.js` source has SHA-256
`296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`.
Its `B8r`/`z8r`/`Enr` derive tool availability from model, project, temporary and
workspace policy. `Pnr` maps the search hint to `forceUseSearch` during official
send preparation. The adapter reuses the committed, filtered menu props, not a
copied policy or the raw hints alone.

Current module: `8b34dbc2-nhot65scqrg20d6p.js`, SHA-256
`36644eb82aac9c399bce384c18140f8c878dd780c8f787440b80f27971729733`.
Its tool store is `OR` (export `Yg`) and setter is `yR` (export `n_`). The old
export names on this build are unrelated: `Ng` focuses the editor and `Bg` is
an initializer. Bindings v5 maps the exact roles; importing the old URL directly
or keeping its aliases would not update the live tool state.

The known owners `Whn` / `Kgn` each have one compiled 265-slot cache. Slot 56
binds the input hints; 63/64/65 reflect disabled tagging, files-only and voice.
Slots 90 and 199 retain the normal tool menu elements with their filtered hints,
lockdown/loading flags and pure `resolveSystemHintBehavior` callback. The new
context module reads only those bounded committed elements. Hidden, local-action,
upsell, duplicate and disabled hints never become native hint writes. This
supports one eligible tool independently, without opening the portal first.

This is an in-page private runtime state bridge, **not** an independent Android
HTTP sender. WebView is still needed for identity, that live state and official
send preparation. The server still enforces tool/account eligibility.

## Transaction rules

1. Resolve only committed React ancestors of the connected composer plus
   button. Reject stale alternates, ambiguous controllers and incomplete props.
2. Reuse the existing text-runtime conversation capture for document, exact
   URL, account/guest proof, controller and conversation; additionally bind model
   and eligible hints. A guest homepage may retain its server conversation ID
   only with positive official logged-out bootstrap and null live session proof.
   Search and image eligibility are independent, not an all-or-nothing catalog.
3. Import only the exact inspected module already observed as a resource or
   module preload. Cache that module within the document. Cold import is
   single-flight, bounded to 1.5 seconds; failure cools down for 10 seconds.
   Warm selection does not wait for a webpage menu or start polling timers.
4. Read `Ng` before any write. Do not change locked, unrelated-tool, connector,
   custom-agent, disabled, login-gated or files-only state.
5. Apply a desired value through `Bg` with `ifPrevSystemHint` and
   `skipComposerAutofocus: true`. Never override locks or focus the keyboard.
6. Confirm the same official signal after mutation and recheck ownership.
   Produce fresh opaque handles for the updated catalog; one immediate receipt
   supports idempotent retry without a second toggle. Stale handles fail closed.
7. Before any mutation, unavailable runtime/schema may use the existing menu
   path. After an attempted mutation, failure is returned without DOM replay.
   A context change during import also cannot fall back into another chat.
8. Dismissing a private-only catalog invalidates handles without a synthetic
   click, Escape or input focus. Already-open official menus keep their existing
   close path, and explicit official controls remain available.

Runtime objects, headers, account identifiers and hints stay in the page closure.
Native snapshots contain only opaque IDs, labels, selected state and semantics.
No capability is declared missing because a DOM or runtime binding is absent.

## Files and verification

- `chatgpt_web_private_composer_tool_context.js`: committed menu eligibility and
  reuse of the existing conversation identity; no second account or draft store.
- `chatgpt_web_adapter_composer_tool_selection.js`: bounded private state bridge
  alongside the unchanged legacy selection verifier.
- `chatgpt_web_adapter_composer.js`: request/select/dismiss integration only.
- `scripts/test-chatgpt-web-private-composer-tools.js`: contract, binding,
  cancellation, idempotency, uncertainty and production wiring tests.

The 2026-09-07 focused Node run passed all 79 cases: the private bridge, previous composer menu/state/
submenu/dismiss policies, private attachment composer (including bundle parse)
and private send observer. It verifies source behavior using fixtures, not a live
private API transaction. Module guards and official runtime access still need
the grouped device check; no latency, temperature or power improvement has been
measured for this candidate.

September 8: 217 focused Node cases passed, including the actual shared resolver,
tool context and unchanged text-runtime regression suite. Guest Search with image
upsell, old/current aliases, warm reuse, login/route changes, committed eligibility,
repeat selection, timeout and no mutation replay are covered. These are offline
fixtures, not proof of a live tool selection or tool-assisted send.

Grouped acceptance: in the production native chat UI, choose search and image
creation, deselect each, switch chats, reopen the menu and send one explicit test
prompt per tool. Confirm the real tool effect and absence of menu touch/reload,
then check an unsupported-runtime case preserves the existing user path. Do not
repeat unrelated verified voice, dictation or conversation-cache research.
