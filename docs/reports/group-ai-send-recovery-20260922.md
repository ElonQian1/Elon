# Group AI Send Recovery (2026-09-22)

## Scope and Evidence

Production Xiaomi `1.1.1800` returned `runtime_unavailable/base_context`, followed by `draft_not_accepted`. The latter is emitted before the DOM submit click. The previous executor treated obtaining server authorization as proof of dispatch, incorrectly turning this known rejection into an indeterminate attempt. See [original diagnosis](group-ai-send-failure-20260922.md).

The 1799 source-card changes did not modify the provider send implementation. The current device's read-only `runtime_assets` probe observed a different public rollout from the supported Sep 21 catalog:

- Anchor: `c2675c8c-m8j393ail312cu4n.js`.
- Shared: `4813494d-cfz7xrlmg5sd5itt.js`.
- Conversation: `conversation-small-g5bv8gqbm1uyfavj.js`.
- Composer: `8b34dbc2-hyspd96sta7ld0cc.js`.
- React: unchanged `2340486e-b3t78qt0psmq9d8c.js`.

This probe is evidence of the current live rollout, not a reconstruction of the already-destroyed failed group document. End-to-end acceptance must verify the isolated group document itself.

## Changes

- Added exact `web_20260922` bindings and fresh-send/recovery admission. No wildcard version imports, guessed exports, credential export, or replay of previous requests.
- Reused the existing public-source AST comparison. All 100 consumed exports are checked against the pinned fixture. Ambiguous aliases are resolved by actual dependency references; the changed composer attachment controller retains the consumed reset/upload contract.
- Recognize only the exact pre-click draft rejection, including its bounded runtime diagnostic suffix, as definitely not sent. Generic rejection, timeout, private unknown receipt, cancellation after authorization, and missing receipts remain uncertain.
- New owner-bound `not_sent` action retires that dispatch token to `cancelled`. It does not reopen the same token. The UI offers a fresh analysis only after server acknowledgement, preserving the same explicit selection and settings.
- Old-token replay cannot cancel a newer reservation; completed and indeterminate requests cannot be retired this way. A lost cancellation response does not automatically resend.
- Source-record cards and the existing continuation/share UI are preserved. Proxy, voice, Cookie storage and login are unchanged.
- Fixed a separate continuation-consent deadlock: after committing the permission change, release the Store connection guard before calling the response reader, which locks the same connection. The selection harness now uses a single mutex-protected connection instead of opening an unrelated connection for every call; recursive acquisition fails immediately in tests rather than hanging.

## Verification

- APK 1802 was published and installed, but the selected-message acceptance still failed before dispatch. A new read-only device probe observed a second rollout (`c2675c8c-irpzu8vf7ngp9dkh.js`), not the earlier asset set. This is not a successful user-path acceptance.
- Added `web_20260922_b` for that exact asset set. All 100 consumed aliases match the earlier Sep 22 contracts; the tools/temporary owners and asset hashes are separately pinned. Unknown rollouts are not allowed by wildcard.
- Public asset hashes and reviewed dependency tests: 66 passed, including all 100 mappings; send/runtime guards: 59 passed.
- Second rollout evidence and temporary/recovery/send guards: 47 passed, no skips (`CHATGPT_RUNTIME_ROLLOUT=sep22b`).
- Android Release unit tests: 30 passed across the group feature, command identity, diagnostics, failure classification, send preparation and session policy.
- Server regression `confirmed_not_sent_retires_token_and_stale_receipt_cannot_cancel_retry`: 1 passed, 2630 filtered out. The broader request suite did not complete and was stopped; it is not counted as passing. The focused test covers retirement, idempotency, stale-token rejection and completed/indeterminate protection.
- Release `1.1.1803`, source `930e5d928`, APK SHA-256 `f3033dad814e37db0e965a58c79533c7bac63b545e7f5427e0013b99e8bdeda1`, published and installed on Xiaomi with data retained. Honor was offline and was not updated.
- Production group acceptance on 1803 selected two synthetic messages through the native UI. Admission became ready on attempt 3, `send_receipt=accepted/authority=same_origin_private`, then `completed` and `deliver_completed`. End-to-end execution took 9.7 seconds; no DOM fallback or automatic resend occurred. The helper selected the earlier identical fixture rows (confirmed from frozen source timestamps), not the newly appended copies; the analysis used a new request, not a replay of the previous token.
- Read-only server verification for request prefix `gaireq_f8a2b8b65`: completed, selected context, two source records, exactly one result message. Sharing remained disabled; no public link was created.
- Native semantic verification: the complete synthetic answer, source-record card and continuation footer are visible. Opening the card shows both selected fixture messages in the native reader; Back returns to the group. The first post-navigation inspector raced a disappearing model control; the helper now waits for the reader explicitly, and the focused open/return check passed.
- The lock-aware harness reproduced the consent regression before the production fix (`database lock already held` at the owner's permission update). After the fix all 8 selection/context/permission tests passed.
- First server publication compiled successfully but stopped before upload because the old live service stopped accepting connections: listen backlog 129/128, 88 CLOSE-WAIT sockets, all four runtime workers in futex wait, including a timed-out loopback health check. A guarded restart of the unchanged old service restored HTTP 200 in 35 ms. No live stack was captured, so the exact triggering call is not proven to be the consent deadlock.
- Server retirement and consent-lock patch deployment: pending in this work batch. Successful APK primary delivery above was verified separately from these server changes.

The two historic indeterminate requests are not automatically retried or rewritten. Never use a private sender's `dispatched=false` alone as proof that a DOM sender did not submit.

## Maintenance

Use `scripts/fixtures/chatgpt-runtime-bindings-sep22.cjs` and `scripts/test-chatgpt-runtime-sep22-public-evidence.cjs` with the actual public assets. Set `CHATGPT_PUBLIC_RUNTIME_DIR`, `CHATGPT_PRIOR_RUNTIME_DIR`, and `CHATGPT_AST_PARSER` as described in the [Sep 21 report](chatgpt-runtime-bindings-20260921.md). Unknown rollouts still fail closed; the analyzer does not automatically approve new bindings.
