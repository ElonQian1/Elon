# Private request lifetime

## Scope

Maintenance of the existing private identity, history, project-directory and
conversation-mutation transports. This is not a new upload, download, share,
model-selection or text-send implementation. Adapter candidate: `263`.

The shared request owner is `chatgpt_web_private_json_request.js` version `1`.
Consumers are auth context `2`, conversation transport `19`, directory `8` and
mutation `5`. Android includes the owner before early identity prewarm; the
desktop bootstrap includes the same asset before early directory observation.
Existing production flags and private/official choices are unchanged.

## Confirmed defects

Both defects were reproduced against untouched base
`5fc6e59ceb01da61779a7dac54881885b592d3f7` with synthetic responses:

- Mutation reconciliation cleared its timeout when response headers arrived.
  A stalled JSON body left the mutation busy indefinitely with no live timer.
- Project refresh returned failure on timeout but left the request running.
  A late body could overwrite a newer successful project directory.

History and auth previously kept an abort timer through JSON parsing, but relied
on the fetch implementation honoring abort and did not bound response buffering.
The shared owner closes those gaps without replaying any request.

## Behavior

- One deadline covers fetch plus response consumption. The returned promise
  settles even if a wrapper ignores abort or AbortController is unavailable.
- Timeout/error/cancellation releases the body reader and aborts its request
  where supported. Late results cannot reach the caller's state update.
- Streamed UTF-8 byte limits are 256 KiB for identity, 1 MiB for a project
  directory, and 4 MiB for history or mutation reconciliation. Declared and
  actually received sizes are checked; split UTF-8 characters are retained.
  Non-streaming compatibility responses get a final byte check and the same
  deadline, but cannot stop buffering before the response implementation returns.
- Invalid/oversized bodies fail explicitly and do not become empty histories.
  Existing cache and explicit official options remain available. Large histories
  beyond the private budget still require the established official path.
- PATCH is sent once. A successful response header remains server acknowledgement,
  not proof of reconciled state. A timed-out follow-up GET releases busy state
  and returns the existing `mutation_server_acknowledged` receipt. No write replay.
- No microphone, WebRTC, transcript, composer, Cookie or app-data reset changes.
  No timers run while the request owner is idle.

## Verification and delivery

- Fourteen focused lifecycle/consumer tests plus five existing script suites pass
  in one Node test run (19 runner entries). Cases include stalled bodies, ignored
  abort, no AbortController, late auth/project responses, UTF-8, byte limits,
  invalid JSON, cancellation and non-replayed mutation acknowledgement.
- The directory source-contract assertion now follows the actual domain
  dispatcher rather than the former giant adapter location.
- Android release-source compilation and all 11 identity/file-index targeted
  tests passed (zero failures or skipped tests). Results are recorded in the
  task's `private-request-lifetime-android` logged command receipt.
- Full APK release and data-preserving installation passed on `v1.1.1540`.
  Windows binary build, device latency and battery/temperature evidence remain
  deferred. This does not claim measured cooling or faster successful TLS.
- [The current work list](web-ai-private-native-remaining-batch.md) records grouped
  delivery evidence and the separate outstanding protocol work.
