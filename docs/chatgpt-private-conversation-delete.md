# Private conversation deletion

Capability candidate: `android_chatgpt_private_conversation_delete_v1`.
Implementation: **partial**, noncurrent conversation selection only.
Delivery: adapter 267 source candidate, no new APK or live deletion acceptance.
Verification: nine new JS checks and existing mutation/directory/asset-bundle
regressions passed; Release source compilation and 68 targeted Android tests
passed (seven new deletion/cache cases). This is not production acceptance.

## Evidence

The current official `conversation-small-hiw4wce20lu6te81.js` asset was read on
2026-09-06. SHA-256:
`296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`.
Its `mYi` deletion mutation has two branches selected by flag `4177111012`:

- `DELETE /backend-api/conversation/id/{conversation_id}` when the flag is on.
- `PATCH /backend-api/conversation/{conversation_id}` with `is_visible: false`
  in the legacy branch.

This candidate implements the evidenced legacy single-conversation branch.
It does not infer the runtime flag, switch endpoints after an error, or equate
archive (`is_archived`) with deletion. The new DELETE branch and real account
acceptance remain unverified. No private conversations were deleted for testing.

## Ownership and native path

- Production sidebar conversation actions show a separate Delete confirmation.
  The existing current-conversation/official route remains available. Current
  conversations are deliberately rejected until navigation and voice ownership
  can be settled safely; this is not a completed current-chat delete workflow.
- UI and MCP use the same consumer port and exact cached conversation selection.
  Confirmation, canonical identity, native current path, and page current path
  are checked. Repeated clicks and existing private mutations share exclusion.
- The versioned module uses the existing page identity and bounded JSON request
  owner. Identity acquisition is limited to 7 seconds; each HTTP request to
  9 seconds. No cookies, bearer values or proof headers are exported to native
  diagnostics. Captured proof headers are not replayed.
- Document, route and account changes before dispatch stop the write. A changed
  context after dispatch produces an uncertain result, never a mutation of the
  new account's directory.
- A transport failure permits one metadata GET, not another write. Only exact
  conversation identity with explicit `is_visible: false` confirms this read.
  Missing metadata, 404 and list absence do not establish successful deletion.
  Unconfirmed writes cool down for 45 seconds; native recovery refreshes the list
  instead of repeating the destructive request.
- Confirmed deletion emits a bounded deletion marker distinct from archive.
  JS and native directory owners reject late rows. Native per-conversation
  snapshots and attachment indexes are evicted for that ID only. Late private
  body/file snapshots cannot recreate it. Deleting the last row emits an empty
  directory rather than leaving stale UI. The last startup snapshot and restore
  URL are also cleared if they still point to the deleted conversation, including
  when the foreground chat is newer than its last persisted snapshot.
  Account-history reset clears markers.

## Verification and remaining work

Targeted JS covers nine cases: exact PATCH/body/header policy, confirmation and
selection, duplicate ownership, account/document/route changes, late success,
uncertain-write reconciliation, 404/HTTP rejection, bounded identity acquisition,
and last-row/late-directory behavior. Android tests cover canonical deletion
events, native directory/file/snapshot invalidation and the shared MCP gate.

Run `scripts/test-chatgpt-web-private-conversation-delete.js`, the existing
mutation/directory suites, and `ChatGptConversationDeletionTest` together with
affected navigation, directory, consumer-port and protocol tests. Passing source
tests is not live protocol acceptance. A grouped production build must verify
one user-confirmed disposable conversation, without touching private history.
Before publishing this deletion candidate, complete explicit active-voice-owner
guarding even when the native UI is browsing a different conversation, plus
current-conversation navigation/voice settlement. The current-page checks alone
do not prove a separate native audio session has released that conversation.
Complete the flagged endpoint selection contract and official sharing separately.
Do not mark this capability
completed merely because the legacy branch compiles.
