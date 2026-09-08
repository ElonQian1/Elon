# September 9 runtime contract update

Capability: `android_chatgpt_private_runtime_bindings_v1`, resolver version 6.
Scope: restore existing verified runtime consumers after a public website build
change. This is not a new transport or completion of stop/follow-up continuity.

## Regression evidence

Normal APK 1589 retained authenticated, ready ChatGPT state. The stopped-followup
acceptance attempt reached `send_receipt` but returned `template_unavailable` and
`runtime_not_observed`: the send succeeded through the explicit DOM path, not
the direct runtime. No stop or follow-up was dispatched. After the synthetic
generation finished, the original blank conversation was restored without
replay, Cookie clearing, or login changes.

The phone's observed Sep 9 assets were newer than both admitted profiles. These
public static files were downloaded without credentials from the official
`https://chatgpt.com/cdn/assets/` path:

| Role | File | SHA-256 |
|---|---|---|
| Anchor | `c2675c8c-k2kd9yafbfvx5mjw.js` | `ac4c5af8a6c19047eb9828d3bf88e6ee921603378d290c899276d7df56920c51` |
| Shared | `4813494d-e0hjx102gn5zjvdh.js` | `5cce48051f230dd8bffc8d2ae20af53af80441ac28d5ba50a6dfedb4a6dc4082` |
| Conversation | `conversation-small-fka464yvjn19vebr.js` | `1c130659c30fda891471cc9c4a8a5c491b595a1ca219516f0f90b0304b9b3c6f` |
| Composer | `8b34dbc2-mx35vjavisrk7hwp.js` | `d2fdfeeb3b225bfa8453f62e208076d3af4005c67cc5133fc91599e989f1a708` |

## Mapping evidence

Acorn parsed both Sep 7 and Sep 9 modules. Comparing declarations and lazy
initializer assignments while preserving literals, operators, property names
and object keys yielded one unique candidate for 50 of 51 consumed exports.
The independent test fixture records all exact aliases; production keeps all
prior build profiles. Similarity is supporting evidence, not automatic
permission to admit a future website build.

The ambiguous async-state store had six structurally identical candidates.
Current official stop function `CN` (export `hHt`) reads `Gd(c)` and its actual
shared-module import is `zS`, resolving legacy `Fx` to `zS`. Its request-active
getter `Gle` imports `Xu`; legacy `Fl` therefore maps to `Xu`. No other matching
store is admitted under these aliases.

Composer owners were checked in the composer module, not an unrelated same-name
function in the conversation module. Old `vqt` maps to `SJt`: identical 30-slot
temporary-chat memo contract and inspected callback. Old `Kgn` maps to `V_n`:
identical 265-slot tool-eligibility contract. The exact callback and module
identities remain required; no reconstructed callback is executed by the app.

## Verification boundary

Offline: 300 targeted cases and the 425-case dependent-consumer suite passed
(overlapping suites, not 725 distinct cases). Device and Release results are
pending at this source checkpoint.

Targeted offline tests cover all 51 identities, cold anchor admission, warm
module reuse, wrong reused aliases, mixed-build rejection, account-document
replacement and real stop/submit consumer composition. Existing Sep 6/Sep 7
tests remain unchanged. Device stop/follow-up continuity must still pass its
separate native four-turn, retained-partial-text assertions.

The stop/follow-up MCP harness now dispatches each state-changing action once.
An uncertain transport result is never automatically replayed as a second send,
stop, or new-chat action. Its 15 focused evidence/dispatch tests passed before
the runtime update; the common MCP helper owned elsewhere is unchanged.
