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

## Verification

- Public asset hashes and reviewed dependency tests: 66 passed, including all 100 mappings; send/runtime guards: 59 passed.
- Android Release unit tests: 30 passed across the group feature, command identity, diagnostics, failure classification, send preparation and session policy.
- Server targeted regressions, Release delivery and real group acceptance: pending in this work batch.

The two historic indeterminate requests are not automatically retried or rewritten. Never use a private sender's `dispatched=false` alone as proof that a DOM sender did not submit.

## Maintenance

Use `scripts/fixtures/chatgpt-runtime-bindings-sep22.cjs` and `scripts/test-chatgpt-runtime-sep22-public-evidence.cjs` with the actual public assets. Set `CHATGPT_PUBLIC_RUNTIME_DIR`, `CHATGPT_PRIOR_RUNTIME_DIR`, and `CHATGPT_AST_PARSER` as described in the [Sep 21 report](chatgpt-runtime-bindings-20260921.md). Unknown rollouts still fail closed; the analyzer does not automatically approve new bindings.
