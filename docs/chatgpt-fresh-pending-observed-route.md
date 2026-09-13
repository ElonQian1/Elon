# Fresh First-Send Observed Route

Status: `acceptance_tooling / offline_verified / device_pending`.
No Android source, private endpoint, default flag or completion scope changes.

## Gap

The native first-send runner retained the synthetic prompt and user-message ID
after an uncertain result, but discarded an already observed provider conversation
URL. The resolver then needed directory pages and candidate navigation even when
that exact route had been available. A cold/stale directory could block lookup.

The existing 1709 handoff was not changed or replayed. This cannot reconstruct a
route which was never retained; that fixture still needs its exact read-only
resolution before a new first-send attempt.

## Implemented

- Retain `observed_path` only from an authenticated ChatGPT provider snapshot
  with one exact controlled first-send prompt and matching UUID user-message ID.
  Require the exact HTTPS ChatGPT origin and ordinary conversation path, with
  no query, fragment, credential component or trailing path segment.
- Capture it while the response is still streaming. It is a lookup hint only;
  the local failure handoff explicitly keeps `readback_completed=false` and
  `replay_allowed=false`. It does not assert native display or terminal history.
- Read that route before any directory request. Only an exact-content miss
  asks for the existing bounded directory search; a read failure is not a miss.
- Deduplicate the observed route against directory rows and count it toward
  the same candidate budget. Legacy handoffs without this field still work.
- Keep exact user identity, expected reply, native/provider route agreement,
  idle ownership, restoration and handoff-hash checks for final resolution.
- Store no new transcript, headers or credentials. The address remains inside
  the existing local controlled-fixture file; reports contain counts only.

Persistence still occurs in the runner's normal finalization, not a crash-safe
transaction journal. No process-kill survival claim is made.

## Evidence

Base: `b83f17844c0c86b6e9095e9fa1f664b5f86b73c2`.

- `fresh-pending-route-baseline-20260914-025953-465` reproduced failure to select
  an observed route without a directory result.
- `fresh-pending-route-resolution-20260914-030438-394`: 32 checks passed,
  including direct-before-directory, lazy fallback, deduplication, bounds and
  rejection of malformed hints.
- `fresh-pending-route-evidence-20260914-030436-375`: send, continuity, cleanup
  and readback contracts passed, plus ten negative observed-route cases.
  A streaming observation is retained but cannot prove completion.
- `fresh-pending-route-trial-20260914-030528-749`: existing trial harness checks
  passed. These are offline synthetic checks, not a phone send or recovery pass.

ADB reported no devices. No APK build, install, message send, real directory
read or conversation change was performed. Use the existing grouped package
and this runner when the phone returns; no APK is required for this script change.
