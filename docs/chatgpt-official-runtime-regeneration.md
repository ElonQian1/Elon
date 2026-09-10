# Official-runtime response regeneration

Capability: `android_chatgpt_official_runtime_regeneration_v1`.
Status: source implemented and offline verified. Normal 1643 / adapter 326 is
installed; its retry passed the corrected runtime identity check but failed at
`timeout_stream_missing`. Contract/runtime 10 and resolver 12 now also observe
the official conversation store without a DOM poll or another request. The 314
related checks pass; adapter 327 delivery and device acceptance are pending.
End-to-end runtime acceptance is not passed. See the
[current evidence](reports/chatgpt-regeneration-store-20260911.md),
the [1637 follow-up](reports/chatgpt-regeneration-observation-1637.md) and
the [grouped delivery](reports/chatgpt-native-regeneration-20260910.md#normal-1630-grouped-delivery).
The original adapter 286 batch wired this candidate into the existing production
`chatgpt_regenerate_response` command. It is an official-runtime bridge, not an
independent Android HTTP generation transport. Do not recreate it while waiting
for the grouped ChatGPT APK acceptance.

## Runtime identity correction

Model contract 10 and regeneration 9 bind observation to the loaded official
session. Existing verified shared exports `H3`, `F5`, and `mq` expose logged-in
state, the session store, and its derived account. Current public source maps
these to `y5`/`po`, `G9`/`In`, and `QG`/`Sw`. `In` reads the session store;
`Sw` derives the account from `session.account` and `session.user.id`, and the
account getters expose `id` and `authUserId`. These are already included in
runtime bindings 11; no speculative endpoint or new export mapping is added.

The owner requires consistent, bounded user/account identifiers from both
official readers. Missing getters, logout, unknown identity or inconsistent
readers reject preparation. Each pre-invocation recheck retains the same owner;
the callback and stream-reset hooks cannot switch accounts before the write.
Observation still checks the route/document, retained conversation object,
server conversation ID, selected new reply and original user parent. True user
or account changes remain unknown/fenced against replay. Request-header cache
hydration, incidental request scopes and token refresh are not account switches
when the authoritative session owner is unchanged. Other model mutation paths
retain their existing header-based admission; no sender or voice path is changed.

The finite red run reproduced false rejection of header hydration and false
acceptance of real runtime identity changes with unchanged cached headers.
`regenerate-runtime-identity-related-20260911-020930-918` passes 236 cases, including
17 focused identity checks and the existing model/reply/branch/writer guards.
Adapter 326 shipped in normal 1643. Its one production retry reached
`timeout_stream_missing`, no longer the earlier account fence. This is not a
completed retry acceptance; the current report distinguishes the remaining gap.

## Official Store Observation

Contract/runtime 10 retain the passive private stream and additionally read the
current official tree when that stream has no visible text. The same new-variant,
selected-leaf, original-parent, user/account, document and conversation checks
apply. Analysis/tool/internal messages, unknown status/content, empty text and
oversized parts cannot confirm a reply. Text stays page-local; this is a command
confirmation path, not a new native transcript transport or independent HTTP.

Resolver 12 exposes the inspected September 10-b `conversationStore` singleton.
A listener is attached only for the pending retry, with no timer/DOM polling or
network request. Completion/document replacement releases both subscriptions;
late matching state can release an unknown-result fence without replay. Older
profiles or an unavailable store retain the existing stream observer. The
existing final deadline also reads the owned tree once; its deadline is not
extended. See the current report for exact source and test evidence.

## Normal 1642 native reentry and remaining owner failure

On 1641 the native model offered retry and message reveal returned success, but
the visible list had no matching row or reply controls. Opening the social AI
friend replaced the RecyclerView adapter with the ordinary friend adapter; the
already-active provider skipped activation and never restored its transcript.
A reversible work/chat comparison on the same APK and synthetic conversation
made the actual retry control appear. Readonly evidence:
`native-retry-1641-layout-20260911-012202-510` and
`native-retry-1641-rebind-proof-20260911-012556-594`.

Source `d3b97c104f5b6eef1fd1149394e3e580439c4aab` now reattaches the active
transcript without restarting its session. An already-bound list is a no-op,
preserving scroll state. Both provider controllers share this presentation fix;
Google transport is unchanged. Twenty-five Release unit tests and six external
UI diagnostic contracts pass. The diagnostics read bounded native view facts,
not conversation text, and do not click controls.

Normal `1.1.1642 (1642)` was published and installed on the trusted Xiaomi;
adapter 325/resolver 11 are unchanged. APK: 40,116,585 bytes, SHA-256
`d3419f33956a0e9d1d3dd32f6aee39869052c695768167d0db6a98b29d8b3b00`.
Release log `webchat-reentry-release-20260911-013930-880` passed in 419.9s.
Run `native-retry-reentry-1642-20260911-014647-575` confirmed the actual row and
enabled retry button without a work/chat toggle, then clicked that button once.
Private model admission passed 6/6 options in both documents. Generation remained
3 across dispatch/capture. `POST /backend-api/f/conversation` returned 200/stream,
but the receipt was `official_runtime_v1:regenerate_unknown:timeout_owner_account`.
No second generation or fallback write was issued; original conversation and
stay-awake state were restored. UI reentry is accepted; runtime completion is not.

Next investigation is the account component of the captured request identity.
The transport can initially expose auth-only warm headers and later observed
conversation headers. This is a source-level hypothesis, not proof that the
live mismatch was harmless. Establish the identity source/transition before
changing the account guard; do not ignore it or replay the unknown write.

## Grouped normal release 1641

Source `2b087cc5282e9cd811f577d7d6da88ff458b054d` passed the normal Release
publication as `1.1.1641 (1641)`, then unattended replacement installation was
verified on the trusted Xiaomi. Adapter 325 and resolver 11 are unchanged.
APK size is 40,116,585 bytes; local APK and live publication metadata SHA-256 agree:
`71d5d14d56e09221ca38305b7597910df3089f89947d32b9b24b72a26f85c93a`.
Log: `retry-native-grouped-release-20260911-001904-311`, terminal pass, 440.9s.

The handset reappeared during publication after the initial ADB inventory was
empty. `native-retry-1641-20260911-002726-085` then stopped at the awake-screen
precondition, before a navigation, awake lease or regeneration dispatch. A
subsequent wake/check found the keyguard still showing. Native retry acceptance
awaits unlock; neither the earlier post-dispatch `timeout_owner_changed` nor
end-to-end private regeneration is marked fixed or completed by this installation.
The publisher's optional LAN firewall and cleanup warnings did not prevent the
verified release/install; task finalization handles repository state separately.

## Current source correction

The native mapper and MCP command both used `message_regenerate`, a flag derived
only from DOM retry/model/overflow buttons, even though execution first uses the
guarded runtime. Native admission now shares `ChatGptWebRegenerationAdmission`:
an identified completed latest assistant and no active generation. The provider
must still declare retry support; Google does not acquire it. Missing composer
or DOM controls do not suppress the entry. This is permission to attempt the
existing command, not proof that the website will accept a write. Its live owner,
parent, model restriction and unknown-result guards are unchanged.

The acceptance script now compares the native message and provider conversation
with the acknowledged synthetic reply, checks its offered actions, and requires
successful reveal before a semantic click. It reports `native_retry_not_offered`,
`native_message_changed` or `native_reveal_failed` before any write, rather than
discarding reveal errors and waiting for a nonexistent button. Thirteen admission
cases and 25 reply/orchestration checks pass, including actual smoke execution
against fake MCP responses and refusal before dispatch. These are not phone passes.

Red run `retry-native-admission-red-20260910-212616-887` compiled the unchanged
production mapper and failed the two DOM-absence UI assertions. After the shared
admission correction, the final Release unit run passed 41 tests in six suites,
with zero failures/errors/skips: `retry-native-admission-final-20260911-001137-716`.
The existing all-command MCP receipt test now deliberately omits the DOM retry
capability; it still dispatches the same guarded regeneration command. Separate
admission tests preserve missing/blank/incomplete/streaming rejection, and the
mapper preserves Google provider restrictions. The unchanged runtime and
production-orchestrator guards also passed 79 tests. Grouped APK delivery is
recorded above; real retry acceptance is not inferred from unit tests.

Regeneration v8 and model contract v9 retain all v7 checks but distinguish route,
document, authorization, account/workspace, device header, retained conversation
object and server ID failures. No private values leave the page. The next real
sample must identify the failed component before any identity policy is changed.
141 focused tests pass, including auth/workspace/device divergence and the
existing model writer, parent, selected leaf and unknown-result fence.

The acceptance runner now distinguishes provider variant identity from a stable
native turn-row ID. A reused row passes only with changed visible text and the
successful runtime receipt, which already proves a new provider UUID under the
original parent. Failed/unknown receipts cannot pass from text alone. Optional
bounded protocol capture starts immediately before retry and is read before
restoration; verification registration requires successful original-view restore.

The runner also requires a fresh readonly page-command ACK after native message
reveal and before either retry dispatch. It rejects a changed/unknown document
generation, changed conversation/turn, nonready bridge, active reply or new draft
or attachment. Eighteen focused cases pass. This is an acceptance safeguard, not
proof of runtime availability or a production fix; it needs no replacement APK.

Contract/runtime v7 retains the same one-shot callback and ownership checks.
The existing final timeout now rechecks the captured stream and official tree
once before reporting failure. A tree commit after the short event-retry budget
can therefore succeed without another stream event, extra polling or a write.
Unknown outcomes remain fenced against replay. Closed timeout suffixes distinguish
missing stream, changed owner, old variant, pending text/tree and wrong parent.
They contain no message content, IDs, account values or exception strings.

Contract/runtime v6 with model contract v8 removes the remaining post-dispatch
model-button lookup. A dispatched reply is confirmed against the captured
document/account, exact URL/server conversation, retained conversation object,
new assistant identity, official selected leaf and original user parent.
The callback still has strict committed-picker and availability checks before
invocation. Unmounted/replaced UI or an absent picker-disabled attribute is not
evidence that an already-dispatched response belongs to another conversation.
No new request, DOM poll, retry or system substitute is introduced.

Eight failing cases reproduced the v5 dependency with streaming/completed replies
after picker detach, unmount, missing disabled state or a throwing DOM getter.
After the correction, 201 related model/regeneration checks passed, including
the production-orchestrator receipt while the model button remains detached.
This code evidence does not identify the full cause of the earlier live timeout.

Contract/runtime v5 separated post-dispatch ownership observation from model
write admission using model contract v7. A known disabled model picker during
generation no longer invalidates an otherwise owned reply. New mutations still
require an enabled picker; document/account/conversation, parent and leaf checks
remain strict. Unknown picker state is not treated as an owned binding.

Normal 1628 reached the production retry button after a completed private-runtime
initial send, but returned `official_runtime_v1:regenerate_unknown:timeout`.
Two disabled-picker observation tests reproduced a code defect; all 109 focused
regeneration/wiring/model tests pass after the correction. This does not prove
that the defect was the only cause of the live timeout. Normal 1629 is published
and installed, but its acceptance stopped before retry: the initial send receipt
passed and the completed-reply predicate timed out. See the
[September 10 evidence](reports/chatgpt-native-regeneration-20260910.md).
The acceptance harness now supports `-UseExistingProbe` on an already open,
strictly identified synthetic test conversation. It records the initial-reply
blocking condition and stops on foreground loss; it does not resend the probe
or count cached/background state as a successful native acceptance. This is a
test-runner correction, not a new APK transport or a passed retry case.

Contract/runtime v4 extends the existing command to the source-observed
`finished_partial_completion` terminal status, while retaining the committed
official `canRegenerateResponse`, model restrictions, same-parent/leaf and
single-writer checks. In-progress and unknown statuses remain inadmissible.
This is not a new sender or a claim of real-device interrupted-turn acceptance.

The retained September 7 conversation asset has SHA-256
`7973d518b083f0f3e23905a279ed019378481bdbdd10fc0196afe9fc7b3b7d35`.
Its `v3i` retry predicate excludes active/new/unauthorized and special turns,
without requiring successful completion; its terminal-turn classification
recognizes `finished_partial_completion`, and its error classifier permits
retry for `finish_details` interrupted by `server_error`. These are inspected
source contracts, not an assertion about a newly observed live response.

The pre-submit hook is now revalidated before resetting the private stream.
Previously a changed model or rate limit was rejected only after clearing the
existing reply. Reset still has its subsequent reentry check because it notifies
listeners synchronously; this does not claim every possible refresh is blank-free.
Seven new regression checks failed against the unchanged v3 implementation.
The corrected regeneration and production-wiring suites pass 45 cases, including
the real private stream parser/merge consumer retaining the previous reply on
hook rejection. The next grouped APK must verify the native retry button on a
completed and an interrupted turn; no new phone result is claimed here.

## Actual contract

The inspected composer module constructs the retry menu's React element children
even while its Radix portal is closed. The committed `dHn` props supply the exact
`retryOption` and `onModelSelect` callback. `iHn` calls that callback with
`sourceEvent` and `requestedModelId`; `CUn` adds the original assistant node and
map-search metadata before invoking the official regeneration hook.

The callback remains responsible for the parent prompt, model, thinking effort,
tool hints, official fresh request preparation and conversation-tree updates. The
native bridge neither reconstructs that POST nor replays captured proof headers.
It does not call React hooks outside a render, open a menu, fill an editor or send
the original prompt as a new user message.

The bridge admits only a terminal text assistant turn (successful or partial), a known server
conversation, one committed retry-menu context, the same current model-picker
conversation and an allowed retry model. It reuses the existing model contract
and the official availability predicate, including live rate-limit restrictions.
Image turns, upsells, unknown runtime versions and other unsupported contexts
retain the existing path before any invocation. New temporary chats without a
recognized server-conversation binding are not covered by this candidate.

## Receipt and ownership

- Bind the account/document, exact URL and server ID, conversation object, selected assistant, callback,
  retry model, original user parent and existing variant IDs. Only the selected
  leaf is eligible; repeat availability and parent checks immediately before use.
- Text submit, private-template send and runtime regeneration cannot overlap
  through the native orchestrator. A duplicate command does not clear the active
  stream, mutate the draft or invoke another callback.
- A callback invocation alone is not success. Reuse the existing private stream
  subscription and require a new, nonempty assistant reply whose ID was not an
  earlier variant, whose conversation matches, and whose official tree confirms
  the same user parent and selected leaf. After dispatch these checks use the
  retained runtime state, not the presence or enabled state of a model button.
- Stream-before-tree races use at most twenty 100ms memory-only checks per
  transaction, including the official selector's missing-node exception before
  a streamed node is committed. There is no new DOM polling, idle timer loop or
  network observer.
- Module import has a 1.5s deadline and uses already referenced pinned resources.
  A failed import may use the existing sender/menu only before invocation and
  while the original binding still matches. Changed context is an unsent failure.
- After invocation, a 15s observation timeout or exception is indeterminate and
  cannot fall back or replay. One listener retains ownership for a late matching
  stream; that stream releases ownership without issuing another command.
  Document replacement releases the old observation. A permanently unconfirmed
  write may require explicit official-page recovery; this remains an acceptance
  case, not a claim of automatic recovery.

The native failure presentation distinguishes unsent rejection, busy and an
indeterminate result. `regenerate_observed` means a new reply has been observed,
not that the entire reply has completed or that independent private HTTP passed.

## Evidence

Public source inspected on 2026-09-07, without account credentials:

- [Composer](https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js),
  SHA-256 `9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`:
  `gHn`/`dHn`/`iHn`/`CUn` and `p4`'s availability/upsell guard.
- [Conversation](https://chatgpt.com/cdn/assets/conversation-small-hiw4wce20lu6te81.js),
  SHA-256 `296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`:
  `Unr` exported as `wpt`, plus `PPt` exported as `f8t`.
- [Shared state](https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js),
  SHA-256 `89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0`:
  thread accessor `sj` exported as `XM` and tree selectors `Z` exported as `HM`.

115 focused Node tests passed across regeneration/production wiring, text submit,
owned attachment submit, the prior private transaction and send observer. The
actual production asset bundle parses. Four pure Kotlin receipt-presentation
tests compiled and passed; this does not cover the full Android consumer build.
The missing-node exception case first failed against the new runtime's initial
implementation, then passed after retaining its bounded confirmation retry.

The original source batch's build/install boundary is superseded by the normal
1629 delivery above. Successful production retry/first-word display, real
closed-portal confirmation, project/temporary cases and account restrictions
remain pending.
Thermal/resource optimization is deferred until functional acceptance. Browser
navigation timed out during the original source batch; no successful
authenticated regeneration was observed in this source batch. Preserve that
distinction in the capability matrix and release notes.
