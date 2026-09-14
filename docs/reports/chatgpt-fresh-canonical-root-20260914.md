# First-Send Canonical Root Reconciliation

## Current Evidence

The 1718 native first-send trial `fresh-new-store1718-20260914-091231-662`
failed in 105.3 seconds. Exactly one native Send click dispatched one accepted
private request and received 31 stream events. Account, document, route and
conversation ownership remained valid; the precise failure was
`store_parent_mismatch`, after the network history had been verified.
No follow-up, replay or automatic fallback send occurred. The uncertain fixture
and its observed provider route remain in the local acceptance handoff. The
awake setting was restored; conversation restoration was deliberately deferred.

## Root Cause And Fix

The pinned `web_20260912` shared bundle exports `yi/VHt`. It applies paginated
messages to the existing `Cx` tree, using that tree's `rootId`, rather than
installing the response's `paginated-root:<conversation>` node. Conversation
`fy` selects this branch for paginated history when an existing `Cx` is present.
The previous test helper replaced the whole tree, so it missed this distinction.

Fresh reconcile v12 accepts the original `client-created-root` only for the
already verified, complete, conversation-bound pagination response when its
server root is absent from the canonical tree. Every hidden ancestor identity,
parent, reciprocal child link, root role and message identity is still checked.
Full-history roots and unrelated canonical roots receive no exception. Requests,
security checks, admission scopes and uncertain-write replay rules are unchanged.
Adapter 388 delivers the repair; new-conversation default remains trial-only
until native first-send and follow-up acceptance.

## Verification

- `fresh-retained-root-baseline-20260914-092029-164` reproduced the first-send
  failure after the test helper was changed to preserve the existing root.
- The helper now returns real parent nodes and correctly normalizes the legacy
  empty root, instead of returning synthetic parent IDs for missing nodes.
- `fresh-retained-root-full-20260914-092249-260` passed 286 Node tests, zero
  failures or skips. Coverage includes first send/follow-up, private streams,
  stop, projects, tools, temporary conversations, attachments, regeneration,
  malformed or changed canonical ancestry, and pinned public-source contracts.
- Public shared/conversation/composer hashes are asserted by
  `test-chatgpt-text-dispatch-public-evidence.cjs`; bundles are parsed, not executed.

Production compilation and device recheck are the next steps. Offline checks
alone do not promote this scope or imply reduced temperature/power consumption.
