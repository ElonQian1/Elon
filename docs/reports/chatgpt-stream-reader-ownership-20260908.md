# Stream reader ownership

## Scope

The [subsequent device run](chatgpt-send-reservation-device-20260908.md) confirms
the narrow stopped-text retention case, but not private stop or next-turn integrity.

This extends the [stop interruption correction](chatgpt-stop-interruption-20260908.md).
It does not replace the sender, stop runtime, WebView identity or voice transport.
The handset was locked during this offline batch; no message or microphone
operation was started. Private-stop device acceptance remains open.

## Reproduced defects

The stream observer checked conversation generation but not the individual
response reader. If another official stream started in the same generation,
the previous reader's late error, EOF or chunk could complete or overwrite the
new reply. Disposal also failed to reject a chunk already awaited by the reader,
and a pending official fetch could acquire an observer after disposal.

These are deterministic source-level reproductions, not proof of the exact
server retry sequence on the phone. Six new cases fail on transport v16; a
separate pending-response case fails before its lifetime guard is added.

## Correction

Transport v17 assigns the active cloned reader one owner. Replacement, native
send preparation, conversation reset and disposal retire it immediately. Late
chunks, EOF and errors cannot mutate or report outcomes for the current reply.
Only the current reader releases its active ownership on completion.

Retirement cancels the observer's cloned response branch without awaiting the
original response's EOF. The official consumer keeps its response and bytes;
the observer does not abort or replay the official request. This bounds obsolete
observer work, but no temperature, memory or latency improvement is claimed
without a device measurement. Adapter target is 305.

## Verification and delivery

- Focused interruption tests cover replacement error/EOF/chunk races, native
  send/reset/dispose boundaries and a late response after disposal.
- A real Node `Response`/`ReadableStream` fixture verifies original bytes remain
  readable and upstream cancellation is never invoked after clone retirement.
- Existing partial-text retention, final-text reconciliation, private stream
  parsing, snapshot scheduling and stop guards are included in the focused run.
- `stream-reader-owner-verified-20260908`: 56 Node runner cases pass, with zero
  failures, cancellations or skips. Source-size and whitespace guards pass.
  This is targeted coverage, not a full Android or device pass.
- Release v1.1.1569 (1569), adapter 305, published from `6cb2245ba`.
  `stream-reader-release-20260908` passed in 459.8 seconds; Gradle reported
  `BUILD SUCCESSFUL in 6m 53s`. APK SHA-256:
  `745e339dd993f9797834f59701f41cf046441a6411b1608690626212172cd597`.
  Publisher reported `APK_ADB_DEPLOY_STATUS=updated`; an independent package
  readback confirmed versionName 1.1.1569 and versionCode 1569 on the trusted
  Xiaomi. No Cookie or application-data clearing was performed.
- Post-install readiness returned `user_action_required` / `unlock_device`.
  No synthetic message, stop command or microphone operation was issued while
  the phone was locked. The next acceptance remains the production stop receipt,
  retained partial reply and successful subsequent send from the linked report.
- Installation and offline success do not close the pending production stop
  receipt, partial-text continuity and subsequent-send acceptance gate.
