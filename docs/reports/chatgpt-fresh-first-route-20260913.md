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
The diagnostic wire version is 8, with strict Android field validation and
compatibility for shipped versions 5-7. Native acceptance readers accept the
reviewed versions explicitly; unknown versions remain rejected.

`fresh-first-route-full-regression-20260913-214627-175` passed 312 Node tests,
zero failures/skips, including pinned-source AST evidence, actual module
composition, route cancellation, parent/branch exclusions, owned follow-up,
existing writing-save regressions and diagnostic allowlists. Resolver tests
passed 16 cases. Existing PowerShell send evidence passed 15 negative-send,
six continuity, nine cleanup and 19 negative-readback cases. An earlier AST
locator test missed a template-literal label; the locator was corrected without
changing the pinned asset or weakening the operation-order assertions.

The final cross-layer Node run, `fresh-first-route-wire-regression-20260913-215629-706`,
passed 313 tests with zero failures/skips, including the actual transaction
output against the same JSON fixtures consumed by Android. PowerShell also
passed 18 negative retry proofs and the existing native identity/draft/cleanup
checks. This caught the missing Android wire-contract update before packaging.
`fresh-first-route-jvm-20260913-215655-901` passed Release Kotlin/Java compilation
and all nine `ChatGptWebFreshTextTrialTest` cases, with zero failures, errors or
skips. The command completed in 330.7 seconds.

## Release And Deferred Device Case

`fresh-first-route-release-20260913-220513-565` published `1.1.1711 / 1711`
from `502c54a45` and nondestructively installed it on the approved Xiaomi in
480.6 seconds. APK SHA-256:
`3391a3e34b5f4253f2d09017800ccba3e4bb12dca004d168f445fab4fea05ef8`.
The unresolved 1709 handoff was copied and byte-verified before the upgrade.
No Cookie or app data clearing, voice action or proxy change occurred.

Read-only recovery on 1711 did not resolve the handoff. The first run stopped
after one cached candidate at `pending_resolution_context_changed`; waiting for
the native route, not only the cached provider route, was added to the harness.
The next run reached an adapter-generation transition, and the final bounded
run `fresh-first-route-pending-bridge-settle-20260913-221656-062` inspected two
candidates but again stopped on changed context. None matched the exact fixture.
No new send/follow-up or replay occurred. The awake lease was restored; the
original conversation restoration was not confirmed. These are interrupted
read-only lookups, not native first-send results or proof of missing server data.

A subsequent read observed the app's home surface, and the next diagnostic
could not start the APK MCP debug service within its timeout. The bounded
foreground query completed with WeChat foreground. Device work stopped instead
of repeatedly reclaiming the screen. The exact guard in the earlier context
change was not captured, so do not attribute it solely to native render lag.

The harness now refreshes its write-idle proof on every candidate, waits for
native route publication, uses the existing pre-dispatch-ready helper for the
trial probe, and preserves its report even if restoration fails. Changed-context
diagnostics contain only booleans. Script-only changes need no new APK.

Next: resolve the preserved 1709 handoff read-only on a stable production surface,
then one native first-send/follow-up case. Normal and project scopes need separate
acceptance. No 1711 first-send pass or default enablement is claimed.
