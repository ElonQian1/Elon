# Send reservation and stop acceptance

## Scope

Production `social_ai` on the trusted Xiaomi, APK 1.1.1569 (1569), adapter 305.
This is a continuation of [reader ownership](chatgpt-stream-reader-ownership-20260908.md)
and [stop interruption](chatgpt-stop-interruption-20260908.md), not a new sender.
Only synthetic text was used. No microphone, login, Cookie clearing, application
data clearing or independent proxy changes were performed.

## Device evidence

- Wireless ADB and unlock were confirmed. Initial MCP `activity_bound=false`
  required opening the production Activity. MCP later continued reporting the
  cached native surface as ready while another application was foreground.
  That first attempt is not valid foreground acceptance. Subsequent attempts
  checked the existing OS foreground predicate, not just cached `ui_state`.
- The native send control returned `control_ok=true` while its draft remained
  present. The official projection had zero messages and no send receipt; the
  native projection had one pending synthetic user row. A diagnostic send through
  the same send owner returned `send_busy`. No accepted-send claim or automatic
  replay was made from the native control acknowledgement.
- Restarting the idle APK without clearing its data removed the old in-memory
  reservation. The subsequent production send returned
  `official_runtime_v1:accepted` and an actual partial reply was observed.
- Stop completed in 72 ms with an empty success detail, meaning the DOM stop
  path was used. The diagnostic reported `stop_runtime_context:request_unavailable`.
  The private stop gate remains failed, not completed.
- Before stop the observed partial text had 333 characters. Three bounded
  samples after stop had 387 characters, retained the observed prefix, were not
  streaming, and stayed in document generation 2. This verifies the immediate
  stopped-text retention case in APK 1569, not every interruption scenario.
- The follow-up action was issued, but the device left the production foreground
  before UI acceptance completed. Later read-only state contained the synthetic
  reply marker, but only one user row containing both test prompts and one
  assistant row. It does not prove an independent next turn or retained history;
  official submission versus native projection needs further diagnosis.
- No further foreground takeover was attempted after that interruption. The
  test context remains available; restoration of the blank conversation is not
  claimed. The stay-awake lease was restored to its original value, 7.

## Source correction

`WebChatSendCoordinator.dispatchReserved` previously returned `NOT_READY` if
readiness disappeared after reservation, without releasing the unsent command
or arming a watchdog. Later attempts were permanently `BUSY`. The UI's pending
render callback and deferred attachment uploads both create this interval.

The exact readiness change on the device was not instrumented, so this is a
deterministically reproduced matching defect, not proof of its precise trigger.
The correction releases only the matching `DISPATCHING` command before any
transport call and returns `REJECTED` with the preserved prompt. A later explicit
send can proceed. Already submitted, accepted or indeterminate requests retain
their existing single-flight and reconciliation protections; no replay is added.

## Verification and delivery

- Red: the two new readiness-loss cases fail on the old coordinator. JUnit XML
  records 14 coordinator cases with two failures, and 10 ledger cases with none.
  The Gradle batch wrapper returned zero despite failed tests, so its outer
  `AI_COMMAND_STATUS=passed` is not used as test evidence.
- Green: `send-reservation-green-20260908` completed Android compilation and both
  JUnit suites: 24 cases, zero failures or errors. The temporary runner additionally
  checked the suite XML rather than trusting only the batch exit code.
- This source correction is not installed in APK 1569. It is queued with the
  previous SharePoint download source batch for grouped release and acceptance.
- Next: confirm real foreground; verify send readiness-loss recovery, private
  stop request ownership, and separate follow-up turns. Preserve accepted voice,
  dictation, read-aloud, Search and prefetch implementations without retesting
  unrelated capabilities.
