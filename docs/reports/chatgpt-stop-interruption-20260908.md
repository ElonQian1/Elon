# Stop interruption correction

## Acceptance boundary

The preceding APK 1567 / adapter 303 acceptance used the DOM stop fallback
(empty success detail). Its immediate native snapshot lost the partial reply.
That is a failed private-stop acceptance, not a completed capability.
See [the device sequence](chatgpt-runtime-owners-20260908.md).

This batch targets those two failures. Source implementation and offline tests
are complete. Android build and device results are recorded below separately.
No microphone, proxy, login, Cookie or application-data changes are involved.

## Corrections

- Stream transport v16 retains received text when its cloned reader rejects,
  including AbortError and network errors. The same turn is finalized locally
  while diagnostics still report `error`, not transport success. It never
  replays a write. New-request generations, navigation reset and disposal still
  discard the previous turn; official final text merges without duplication.
- Production composer lookup now includes the fixed `#prompt-textarea` ID.
  The current public composer bundle uses this ID, also used by the existing
  private tool-context integration. The previous fallback relied on a test ID
  or `contenteditable=true`; an ID-only disabled rich editor was missed. The
  focused fixture proves this gap and its correction, but does not establish
  that it was the only cause of the 1567 device fallback.
- Stop runtime v4 records bounded stage codes. The existing semantic MCP probe
  accepts `stop_runtime_context`, returning only an allowlisted code from the
  last stop. It does not start a stop, read message bodies, import modules,
  issue network requests or poll the DOM. Native receipt validation rejects
  unknown values. Probe v15 upgrades existing v13/v14 observers without stacking
  fetch hooks. Adapter target is 304.

The stop operation itself remains the same verified official runtime call, with
exact current request, identity, committed owner and voice exclusion checks.
No eligibility or post-invocation retry guard is weakened.

## Verification

- `stream-interruption-red-20260908`: both partial-retention cases fail on the
  old code. `stream-interruption-tests-20260908`: 45 Node runner cases pass,
  including seven new interruption cases and existing stream/stop coverage.
- `stop-composer-red-20260908`: the ID-only disabled composer case fails on the
  old production selector. `stop-composer-tests-20260908-verified`: 70 Node
  runner cases pass, including existing guest/request/voice guards and new
  diagnostic tests. These two suites overlap and are not 115 unique tests.
- Native diagnostic validator `testReleaseUnitTest` passes; the logged batch
  `stop-diagnostic-native-tests-20260908` completed in 326.6 seconds.
- Release build/publication: pending.
- Device: trusted Xiaomi remains online, but readiness returned
  `unlock_device`. No stop test was replayed while it was locked.

## Next acceptance

Use production `social_ai`, one synthetic long response and the semantic stop
command. Record the matched receipt, `stop_runtime_context`, stream revision,
message counts and partial-text continuity before/after stopping. Sample a
bounded interval after stop to distinguish transient projection gaps from loss.
Only `official_runtime_v1:stop_observed` plus retained text and a successful next
send satisfies this narrow gate. Restore the original safe surface, and do not
repeat accepted Search or ordinary-send research.
