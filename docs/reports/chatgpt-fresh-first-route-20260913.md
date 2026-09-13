---
capability_id: android_chatgpt_fresh_new_conversation_text_dispatch_v1
implementation_status: partial
verification_status: offline_passed_device_pending
production_default: false
---

# First-Route Reconciliation Evidence

This report supplements [the first-send contract](../chatgpt-fresh-new-text-dispatch.md).
It does not change existing-conversation defaults or authorize replay.

## Exact Pending Recovery

`fresh-pending-native-readback-verified-20260913-212748-918` resolved the preserved
1708 controlled fixture in 8.4 seconds on release 1709. It read the production
native cached directory through MCP, not only the eight visible rows. The cache
contained 201 rows; the first of 20 bounded candidates matched the exact user
message ID and controlled prompt. Provider/native routes and the completed
assistant marker also matched. No message was sent or replayed. The fixture was
left open explicitly and the awake lease restored.

`resolve-chatgpt-fresh-pending.ps1` now preserves this reusable read-only workflow:
strict local handoff validation, bounded cache pagination, idle/draft/voice
guards, exact user and native reply proof, compare-before-write handoff update,
and guarded restoration. Activity dates only order candidates; they never prove
identity. A cached directory can be incomplete. Native readback has a bounded
publication wait instead of declaring failure while UI projection catches up.

## Actual First-Send Failure

`fresh-pagination-native-first-followup-20260913-212816-170` ran one native
new-conversation send on 1709, with no runtime seed. Private HTTP was accepted
and 33 owned stream events arrived, including input-message, handoff and
stream-complete events. After 105.9 seconds the writer remained pending with
`history_reconciliation_required` and history `owner_changed`. No follow-up or
replay occurred. The new exact handoff remains preserved locally; the awake
lease was restored. A later read still showed the official home route, two
messages and empty drafts, not a completed first-send reconciliation.

## Source Repair And Bounds

Pinned `web_20260912` public source shows `C1t`'s `handleResponse` binding the
server ID and calling the official navigation helper before reducing message
events. Reconcile v10 follows this order for non-temporary first sends: settle
owned navigation, recheck ownership, then read and hydrate authoritative history.
Rejected/cancelled navigation cannot start hydration or release the writer.
Existing/temporary paths retain their prior order. No owner, branch, root,
account or privacy check is relaxed. This order mismatch is concrete source
evidence, but the precise device `owner_changed` guard is not yet established.

Context v9 and transaction v19 expose only bounded ownership/reconciliation
guard labels in the existing trial diagnostic. They contain no account values,
conversation IDs, URLs, tokens or message contents. Adapter version is 380.

`fresh-first-route-full-regression-20260913-214627-175` passed 312 Node tests,
zero failures/skips, including pinned-source AST evidence, actual module
composition, route cancellation, parent/branch exclusions, owned follow-up,
existing writing-save regressions and diagnostic allowlists. Resolver tests
passed 16 cases. Existing PowerShell send evidence passed 15 negative-send,
six continuity, nine cleanup and 19 negative-readback cases. An earlier AST
locator test missed a template-literal label; the locator was corrected without
changing the pinned asset or weakening the operation-order assertions.

Next: preserve the unresolved 1709 handoff byte-for-byte, publish one candidate,
resolve it read-only, then one native first-send/follow-up case. Normal and
project scopes need separate acceptance. No device pass or default enablement
is claimed by the offline tests.
