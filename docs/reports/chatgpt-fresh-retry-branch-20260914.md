# Independent Retry Branch Reconciliation

Current: grouped build 1726 (adapter 394) is published and its native acceptance
passed; see [device evidence](chatgpt-fresh-retry-native-1726.md). The earlier 1725
result remains failed. The source-batch account below is retained as history.

## Evidence And Gap

The [1725 device report](chatgpt-fresh-retry-admission-20260914.md) established
one accepted independent retry, 25 owned stream events, and
`history_reconciliation_pending / store_not_reconciled`. It did not identify
the exact live leaf mismatch. No request is replayed by this batch.

The pinned `web_20260912` conversation asset, already retained under the hashes
in the admission report, exposes `BEn / gy` history loading through `fy`. The
`fy` branch policy preserves an existing current leaf if it is not an ancestor
of the server-selected leaf and still exists in the merged tree. A regenerated
answer is a sibling, so loading its history need not select it.

The shared asset exposes `sY / Zx`, a conversation-scoped state transaction, and
`KJ / sS.setCurrentLeafId`. The latter calls `updateTree`, increments
`_treeVersion` and selects only the requested leaf. These exact exports were
already version-mapped for writing-block transactions; this batch reuses them,
without creating another runtime binding, replacing the store or loading DOM
controls. The public-source test verifies hashes and these definitions by AST;
downloaded provider JavaScript is not executed.

The former fixture always selected the history current leaf. It therefore
proved a simpler state transition than the official sibling-preserving loader.
The fixture now supports that observed policy and the end-to-end transaction
test exercises it explicitly, rather than merely weakening the success check.

## Correction

- Regeneration context v5 requires the reviewed state action before dispatch.
- Reconciler v13 retains the server leaf ID only after the existing authoritative
  history checks confirm this command's observed, terminal assistant reply.
- After hydration, a guarded official transaction may move from the original
  answer to that verified reply. An already-selected reply is a no-op.
- The original prompt, ancestor, account, document, conversation, request
  ownership, terminal status and reply-parent relation are rechecked, including
  inside the state transaction. A user-selected different sibling prevents
  history application as well as later branch selection.
- An unrelated device's response, missing/partial hydration, active server
  generation, aborted request or retired document cannot settle the ledger.
- Stop and read-only recovery reuse this same path. No second POST, runtime
  retry callback, draft clearing, reload or whole-store replacement is added.
- Existing closed diagnostics now distinguish unsettled status, leaf mismatch
  and prompt mismatch instead of reporting only `store_not_reconciled`.
- Adapter 394 loads the correction; the Kotlin change is only that constant.
  Ordinary send, writing editor/export, voice and proxy code are unchanged.

## Verification

- `retry-branch-red-20260914-164018-028` reproduced the stale-sibling failure
  and missing state-action admission guards before the runtime correction.
- `retry-branch-verified-20260914-164154-032`: 82 tests passed, zero skipped.
- `retry-branch-shared-regressions-20260914-164305-506`: 140 shared tests passed.
- The first combined run found an older parent-shape test applying an unobserved
  reply. The test now explicitly proves that history alone cannot authorize it,
  then supplies the owned stream frame before checking runtime parent mapping.
- `retry-branch-complete-contract-20260914-164701-490`: 223 tests passed,
  zero failures/skips, 12.7 seconds. This includes pinned public source, context,
  request, stream, stop/recovery, history, runtime bindings and native wiring.
- Final ownership-order correction and the added retired-document case:
  `retry-branch-owner-final-20260914-164907-405` passed the affected suites.

No Android compilation, APK publication or corrected-device pass is claimed in
this source batch. Group delivery is intentional; offline fixture coverage does
not prove that the original phone failure has no additional cause.

## Device Preservation And Next Acceptance

A bounded MCP state check still found build 1725/adapter 393, two test messages
and native streaming true. One `chatgpt_refresh_controls` snapshot, not a page
reload, exposed no previous/next-response control in its 15-control manifest.
No control was guessed or clicked, and no write or navigation followed.

The older isolated retry remains pending with its controlled draft intact. Do
not install over, force-stop, reload or clear that ledger to fabricate recovery.
First resolve its known server result through an existing authoritative recovery
path, or retain it as an explicitly unconfirmed prior trial. Then accept the
grouped correction with one native retry, exact owned terminal reply, pending
false, preserved draft, and no duplicate user message or second POST. Only a
complete native pass can promote `android_chatgpt_fresh_regeneration_v1`.
