# Official-runtime response regeneration

Capability: `android_chatgpt_official_runtime_regeneration_v1`.
Status: source implemented and offline verified; not device verified or completed.
The original adapter 286 batch wired this candidate into the existing production
`chatgpt_regenerate_response` command. It is an official-runtime bridge, not an
independent Android HTTP generation transport. Do not recreate it while waiting
for the grouped ChatGPT APK acceptance.

## Current source correction

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

- Bind the account/document, conversation object, selected assistant, callback,
  retry model, original user parent and existing variant IDs. Only the selected
  leaf is eligible; repeat availability and parent checks immediately before use.
- Text submit, private-template send and runtime regeneration cannot overlap
  through the native orchestrator. A duplicate command does not clear the active
  stream, mutate the draft or invoke another callback.
- A callback invocation alone is not success. Reuse the existing private stream
  subscription and require a new, nonempty assistant reply whose ID was not an
  earlier variant, whose conversation matches, and whose official tree confirms
  the same user parent and selected leaf.
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

Grouped APK build/install, production retry/first-word display, real closed-portal
binding, project/temporary cases and account restrictions remain pending.
Thermal/resource optimization is deferred until functional acceptance. Browser
navigation timed out during the original source batch; no successful
authenticated regeneration was observed in this source batch. Preserve that
distinction in the capability matrix and release notes.
