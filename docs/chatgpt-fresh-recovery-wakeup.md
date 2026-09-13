# Fresh Text Recovery Wakeup

Status: `implemented / offline_verified / grouped_release_pending`.
This repairs lifecycle recovery within the existing private sender. It does not
promote fresh first-send, tool, project, attachment or temporary scopes.

## Reproduced Gap

A completed HTTP reader can retain an unresolved writer until terminal history
is confirmed. Recovery already joined concurrent reads, used a ten-second
cooldown and limited automatic recovery to three jobs per turn. However:

- An `online`, `pageshow` or foreground event during cooldown returned deferred
  and was forgotten. Nothing ran when cooldown ended.
- The same event during an in-flight history read only joined that job. If the
  old read then failed, the restored connection never triggered another read.

Both gaps were reproduced with the actual transaction/recovery modules, fake
time and synthetic history. These are code-level race reproductions, not proof
that either gap caused a particular past phone/network incident.

## Change

Recovery v2 retains one observed resume event per exact writer. It waits for the
existing job and cooldown, then requests recovery through transaction v22. The
same native completion/reconciliation path is used; adapter version is 386.

- Repeated events merge into one wakeup; failed wakeups do not start polling.
- Hidden/offline events cancel a scheduled or queued wakeup. A later foreground
  or online event can request another, within the unchanged per-turn budget.
- An explicitly offline automatic attempt consumes no history reads or budget.
- Before the delayed read, recheck visibility, connectivity, exact document and
  conversation ownership, stop state and terminal confirmation.
- Manual recovery, Stop, document cancellation and writer retirement remove the
  pending wakeup. A stopped or superseded turn cannot update another conversation.
- Only existing private history reconciliation is invoked. No new endpoint,
  prepare, security-token request, text POST, draft clearing, WebView reload or
  fallback replay is added. Existing microphone/audio behavior is untouched.

## Evidence

Base: `31ac20d25a2acc5fe54ec59f217c2be974597f66`.

- `fresh-recovery-wakeup-baseline-20260914-024536-734` reproduced a missing
  fourth history read after a cooldown-time online event.
- `fresh-recovery-inflight-baseline-20260914-024907-208` reproduced a missing
  second read when online arrived during a failing in-flight read.
- `fresh-recovery-wakeup-regression-20260914-025031-010`: 185 tests passed,
  zero failures/cancellations/skips. Includes production-default coordinator
  wiring, single-flight resume, follow-up admission without replay, background,
  offline, document replacement, stop/recovery boundaries, stream handoff,
  new/temporary contexts, regeneration and attachment non-regression.

ADB reported no connected devices. This source batch has not been Android-built,
published, installed or tested against a real network loss. The already verified
1715 APK predates it. Include one pending-turn foreground/network-return check in
the next grouped native UI acceptance, preserving the existing unresolved-first-
send fixture rather than replaying it. Explicit refresh remains available after
the automatic budget is exhausted; the budget is not reset by event storms.
