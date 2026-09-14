# First-send 1716 reconciliation evidence

## Device result

On 2026-09-14 the Xiaomi device was available over USB. The older uncertain
1709 first-send fixture was resolved through exact user-message identity and
native/provider readback, with no send or replay. The resolver now waits for
the requested route, current adapter, authenticated bridge and idle nonempty
history together; a transitional cached route is not considered ready.

`fresh-pending-settled-device-resolution-20260914-075857-052` passed in 29.4s:
213 cached rows, 5 directory reads, 8 inspected candidates, exact user match,
original route and awake state restored, 0 sends.

The retained official 1716 APK was installed using `adb install -r` without
clearing application data or Cookie state. SHA-256:
`e2efbed7642a29332982ed0fc9d0564676b4aa387106995ce8987cdc89ffa434`.

`fresh-new-verified-origin-1716-20260914-080138-318` failed after 104.4s.
There was exactly one first-send click and no follow-up. The request was
accepted, 32 stream events arrived, and history ownership was confirmed;
canonical page state still reported `store_not_reconciled`. This is not a
successful end-to-end independent sender. The controlled pending fixture is
retained locally with replay disabled. No private message content, route or
credentials are included here.

## Source correction

The page's history-parent diagnostic already emitted the v2 pagination
descriptor, but Android still rejected it as invalid evidence. Android now
accepts the strict four-boolean descriptor and `paginated_root`, while retaining
the legacy shape and rejecting extra fields, coercion and inconsistent states.

Store reconciliation now distinguishes ownership, history loading, async state,
attachments, user identity, parent chain, leaf and prompt conflicts. These are
bounded reason codes, not content. The request contract and success predicates
are unchanged; this instrumentation does not itself fix the remaining send.
Adapter version: 387.

## Verification

- `fresh-pending-navigation-regression-20260914-075854-545`: 44 checks passed.
- `fresh-store-diagnostics-node-20260914-081500-202`: 144 tests passed, no skips.
- `fresh-store-diagnostics-jvm-20260914-081534-985`: Release Kotlin/Java compilation
  and 17 focused Android tests passed, no errors/failures/skips, 306.4s.

## Next boundary

Read back the existing pending fixture without replay after the grouped update.
Do not infer the exact store failure before receiving the new reason code.
Writing Blocks remains a separate production capability; its existing verified
local editor/export and ordinary cloud-save paths must not be reimplemented
because this experimental first-send acceptance failed.

The diagnostic update was published and installed as 1717. Before resuming
Writing Blocks acceptance, `fresh-1716-readonly-resolution-20260914-083331-873`
confirmed the existing pending fixture in 11.5s: one inspected candidate,
exact user/native response readback, 0 sends, awake state restored and the
confirmed conversation left open. The fixture is resolved without replay;
the independent first-send store reconciliation itself is still not accepted.
