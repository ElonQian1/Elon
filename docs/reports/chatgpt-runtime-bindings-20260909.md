# September 9 runtime contract update

Capability: `android_chatgpt_private_runtime_bindings_v1`, resolver versions 6/7.
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

## Installed 1590 and the second observed build

Release `1.1.1590`, source `cd320208b768b693cad9d9662d3ee8ed9433a7d9`, passed
the normal publisher and unattended replacement installation. Local APK and
online manifest SHA-256 both equal
`fd969c2dddbb5142c1ecffc38bc18d6a73837fccfff90562d3c4f75d9697d7eb`.
Evidence: `runtime-bindings-sep9-release-20260909-062330-912` (458 seconds).

The next production acceptance again stopped at `send_receipt` with
`runtime_not_observed`, before dispatching any stop or follow-up. A fresh on-page
inventory now showed `c2675c8c-lz0unwv5yke95cwv.js`, a different official build.
This demonstrates the first admitted Sep 9 profile was insufficient for the
reopened page; it does not prove a login failure or a broken audio path. The
one synthetic send finished, then the original blank view was restored. A
subsequent settled snapshot was authenticated/ready with zero messages/draft.
Evidence: `stopped-followup-device-1590-20260909-063148-582`.

Version 7 adds `web_20260909_b` without replacing the first profile. Its actual
anchor imports and public source hashes are:

| Role | File | SHA-256 |
|---|---|---|
| Anchor | `c2675c8c-lz0unwv5yke95cwv.js` | `0508bf7d70f5269217002af761c50533c61c451ebfc1ce8ea94c7d66f1e20555` |
| Shared | `4813494d-bgyv5408fxme7xxv.js` | `72ed87dd6d8a5241abac73d9c720f8e92bf87bbee7980331fcf042fea64d14ca` |
| Conversation | `conversation-small-hg48c5uox88r7a00.js` | `7bfb494a2d582faba39c835c20feced3d1c81805835f7e3d97aa9ce0c713060a` |
| Composer | `8b34dbc2-cj4kfo18e1ldvw16.js` | `c24245fc260253db68ae531c586174a3eeff95e694b7c8a0e76eb77eae5fe906` |

The same AST procedure produced 50 unique candidates. Official stop `Nln`
reads `Id(c)` (import `zS`), active request `Mle` (import `Xu`) and enum `ml`
(import `met`), resolving the remaining store and enum identities explicitly.
Composer owners `DJt`/`H_n` retain the same inspected 30/265-slot contracts.
Both Sep 9 profiles execute the same independent identity and consumer tests;
no uncertain message was replayed, and the full continuity gate remains open.

## Normal 1591 authenticated acceptance

Release `1.1.1591`, source `b609e779b07f8ce318ca575d19fd6022c8e43dc4`, passed
the normal publisher and unattended replacement installation on the trusted
Xiaomi. Local APK and online manifest SHA-256 agree:
`f43d1d0d16b99f00cc45ba06a923bcabd2207b3b0469ae5414ca17ee6bec6aa1`.
Build evidence: `runtime-bindings-sep9b-release-20260909-064116-993` (438.1 s).
Both Sep 9 profiles and dependent consumers pass 431 Node cases, no skips.

The 96-entry public asset inventory was truncated and omitted all known role
filenames on this reopened page. A local admission check using that incomplete
inventory was inconclusive, not evidence of an unsupported build. The actual
official composer-tool context subsequently reported `ready`, with no selection
write. Future diagnostics must not equate bounded inventory absence with a
runtime failure.

Acceptance scope `ordinary_authenticated_native_stop_followup`: **completed**.
Reuse this accepted path unless there is new regression evidence. The unchanged
native consumer flow completed in 30.7 seconds:

- Both sends were acknowledged as `official_runtime_v1:accepted`.
- Mid-public-answer stop returned `official_runtime_v1:stop_observed`.
- All 229 characters observed before stopping were retained after the follow-up.
- Exactly four ordered native rows remained: user, assistant, user, assistant.
- Both prompts stayed separate; the final follow-up answer matched its marker.
- Native/official drafts were empty; original blank chat and awake lease restored.
- No microphone, system TTS substitution, private-content output, login clearing
  or independent proxy modification was involved.

Evidence: `stopped-followup-device-1591-20260909-065407-624`. This establishes the
current authenticated production UI path, not the cause of every earlier guest
projection failure. The previous guest continuity case remains unretaken; do
not clear the user's login to recreate it, or report all private features done.
