# Fresh Send Early Stop

Status: `implemented / offline_verified / grouped_release_pending`.
This is a regression fix within the existing personal plain-text sender, not a
new capability completion or a fresh-regeneration promotion.

## Failure And Fix

The request can be dispatched before its user message appears in the next
history GET. The previous stop coordinator returned `stop_owner_unconfirmed`
immediately when history still ended at the unchanged completed parent. It did
not send a stop request, although the owned send was active.

Reconciliation v7 identifies only that propagation gap. Stop v2 performs at most
three ownership reads, separated by 200 ms, inside its existing 12-second
deadline. Once the exact submitted user appears on the owned branch, the
existing one-use conduit/turn stop request is allowed. A different conversation,
project, privacy scope, branch, unknown async state or changed owner is not a
propagation gap. New-conversation and regeneration operations are excluded.

An absent user after the bounded reads remains unconfirmed. A timeout, account
or context change ends the wait without a late POST. A lost POST acknowledgement
does not authorize replay or release a user-only writer; subsequent recovery is
read-only. Existing native partial text, exact terminal-history reconciliation,
and stopped-user follow-up parent ownership remain intact.

Transaction factory v17 picks up the new components when idle; it never replaces
a pending writer during adapter reinjection. Initial composer-free ownership is
not implemented by this change.

## Evidence

- Base: `8eb232a7c17199bc71d8e8a466dcbe153cf3ca3a`.
- `fresh-early-stop-baseline-20260913-154414-672`: reproduced premature
  `stop_owner_unconfirmed` using an absent-then-visible submitted user.
- `fresh-early-stop-verified-20260913-155704-562`: 145 tests passed, zero failures
  or skips. Includes stop, reconciliation, real transaction composition,
  follow-up, recovery, stream delivery, production wiring, regeneration
  non-regression and retained public-source contracts.
- The transaction test now covers partial assistant, user-only, and initially
  absent user histories; each dispatches one stop and can start an owned follow-up.
  Timeout and unknown-result tests assert no late or duplicate stop.
- Reviewed public `conversation-small-h1dtzoris1y9588z.js`, SHA-256
  `da08c64c132306779e09ba89cac64fa560b120e7560ffdc29b3ce5a0b8ccd67e`,
  export `xWt` / local `KM`, is the existing source for stop endpoint, gates,
  async exclusions and turn headers. It was AST-inspected, never executed.
  The bounded propagation wait is our coordination policy, not a claim that the
  official implementation uses the same retry timing.

The device was unavailable during this source batch. No new APK build,
installation, live send or stop was performed. The prior 1699/1701 device
evidence does not verify this new timing case. Include it in the next grouped
production native-UI acceptance; preserve Cookies, data, voice and proxy state.
