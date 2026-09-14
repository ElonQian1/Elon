# Fresh History Recovery Suspension

Status: implemented / offline_verified; production/device acceptance pending.
Base: `f8499a86554b670c457a24b2105ce7a71580f701`. Adapter 412,
recovery 3 and transaction 31 extend the existing sender; no new transport.

## Reproduced Gap

Background/offline events cancelled a queued cooldown wakeup but left an active
automatic history read running. Further reads in the same job also ignored
visibility/connectivity changes. A document whose cancellation signal had
already fired could start another read if its identity callbacks still matched.

`fresh-recovery-suspend-baseline-20260915-062556-999` reproduced these gaps
with the production recovery/transaction modules. This is deterministic code
evidence, not a diagnosis of an earlier unspecified phone incident.

## Change

- Cancel the automatic read's AbortController on background/offline events.
  Preserve the submitted writer, message identity and unknown-result barrier.
- Check visibility/connectivity between attempts as well as at admission.
  Check an already-aborted document before admitting any read.
- Retain a subsequent observed resume through cancellation and cooldown.
  Existing single-flight, three-job budget and ten-second cooldown are unchanged.
- Ignore late cancelled results. A successful later read follows the existing
  exact-message/branch reconciliation and native completion path.
- Do not cancel explicit manual history checks, live audio, streaming writes or
  Stop commands. Do not POST again, clear drafts, reload WebView, reset login,
  alter the proxy or turn a temporary empty DOM into missing capability.

## Verification

- 68 focused recovery/transaction cases passed, including suspension during an
  active read, immediate foreground/online return, late-result rejection,
  one original POST, manual-check independence and pre-cancelled documents.
- Wider fresh-text/Canvas regression: 764 passed, four optional skips, zero
  failures; `fresh-recovery-canvas-final-regression-20260915-063225-650`.
  One existing assembly assertion still expected orchestrator 12 while the
  accepted source was already 13; the assertion now checks the actual version.
- Xiaomi production baseline 1748 / adapter 411 was authenticated, idle, on the
  native social AI page, without a draft, active voice/dictation or pending trial.

## Remaining Boundaries

The September 15 compatibility and no-composer first-send acceptance on 1748
are already complete; reuse them. Canvas edit/save/history/restore are implemented
but an original editable fixture is still missing, so they are not device-accepted.
Cold bootstrap without initialized identity/runtime, process recreation and
actual network-loss recovery remain separate acceptance cases. A foreground
round trip alone does not prove any of those or a thermal improvement.
