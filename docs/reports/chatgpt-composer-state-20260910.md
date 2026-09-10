# Native composer state, September 10

Scope: production ChatGPT model levels, quick-tool selection and temporary-chat
receipt attribution. This is not independent Android HTTP submission or a claim
that all remaining private functions are complete. Reuse the accepted audio,
captions, dictation, directory and model-version selection scopes.

## Observed regressions on normal 1625

- The visible native SeekBar accepted Accessibility `ACTION_SET_PROGRESS` from
  level 3 to 2, but the official model and selected private catalog entry stayed
  at level 3. No model-write command appeared. The renderer only dispatched in
  `onStopTrackingTouch`, which keyboard/accessibility changes do not call.
- Authenticated native Tools -> Create Image selected the private tool and
  showed its active chip. The chip's close button then produced
  `official_tool_runtime_v1:accepted`; a fresh private catalog had no selected
  tool, but the native chip still remained. The renderer copied only non-null
  selections, so an explicitly deselected catalog could not clear stale UI.
- Earlier tool-command timeouts occurred while another package was foreground.
  They are not evidence of a tool-protocol failure. Back in the production chat,
  the same installed runtime returned both authenticated private tool options.

The existing two-message conversation, empty draft and document generation were
preserved. No messages, uploads or microphone sessions were started.

## Changes

- `47918e113`: native level input commits keyboard/accessibility progress;
  touch drags still commit only their last user position on release. Programmatic
  updates, invalid values and repeated release callbacks cannot submit. Existing
  semantic acceptance gains model-level, tool-close and temporary-menu actions.
- `371bf87a3`: reconcile explicit tool deselection into the native chip. Missing
  catalog remains unknown and preserves the prior display; a nonempty observed
  catalog with no selected quick tool clears it. No parallel tool store or writer.
- `03d017cc8`: temporary runtime v8 marks confirmed callback writes as
  `official_temporary_runtime_v1:accepted` and already-observed no-op state as
  `official_temporary_runtime_v1:unchanged`. Compatibility and failed paths cannot
  claim these markers. Transaction behavior is unchanged.

The other worktree's uncommitted `webChatLastCommandStatus` accessor is unrelated
to the composer-renderer lines; it was preserved and not included.

## Offline evidence

- Android `testReleaseUnitTest`: 26 tests, zero failures/errors/skips, including
  actual model-level, model-presentation, quick-tool and coordinator tests. Main
  production Kotlin and test Kotlin compiled successfully.
- Node batch: 56 cases passed. The strengthened private temporary receipt suite
  then passed all 49 cases, including no-op, failed transition and fallback
  attribution. These 49 are a subset, not additional unique cases.
- External Java semantic harness compiled and executed on 1625; it uses native
  accessibility actions and exact package/role/range guards, not coordinates.
- Source-size, staged-document and whitespace checks passed before commits.

Command-log stems: `composer-state-jvm-20260910-073924-786`,
`composer-state-node-20260910-074320-458`,
`temporary-receipt-tests-20260910-074402-378`.

## Grouped device acceptance

The 1626 preparation failure below is historical. The 1627 checkpoint resolves
the tested navigation/model/temporary cases; quick-tool follow-up remains open.

Normal 1.1.1626 (1626), source `03d017cc8`, passed the Release publisher and
replacement installation. APK SHA-256:
`a0b27a04f87d336da7e5f8b3da70b95f806181f845066d7e48dcdc49f54906ab`.
The publisher verified the remote artifact and installed build without clearing
application data or login. Log: `composer-state-release-20260910-074815-357`.

The corrected device cases are **verification deferred**, not passed. Both
grouped attempts stopped during preparation, before model/tool/temporary writes:

- Cached native readiness was not current WebView readiness. Fresh state had an
  authenticated ordinary conversation and two retained messages, but no ready
  composer, model label or temporary-chat control. The adapter bridge was ready
  while the native connection state was error; these are different signals.
- The native recovery banner reported a connection error. One actual native
  Retry action briefly hid it, but after a bounded 30 seconds the error and
  missing composer remained. Available log samples did not establish a network
  or renderer-crash cause; absence of such markers does not prove either healthy.
- The second attempt confirmed restoration of the original conversation,
  unchanged message count and empty draft. Both attempts restored the awake
  lease. No microphone, message sends or application-data clearing occurred.
- Wireless ADB remained usable: a bounded remote serial-number read succeeded.
  ADB connectivity is not evidence that ChatGPT's page is usable.

Attempt logs: `composer-state-device-20260910-075558-691` and
`composer-state-isolated-device-20260910-080051-402`. The external semantic
harness now exposes the native recovery banner's public message and its Retry
control, so this failure can be diagnosed without screenshots or conversation
text. This harness-only follow-up does not change the published APK.

The original prerequisite was current-document recovery, not feature discovery.
The follow-up below used that evidence rather than repeating blind retries.

## Normal 1627 recovery and acceptance

- `3eafd5ffb`: new-chat intent can proceed from a recoverable current document
  even when the native connection state is error. The official transaction still
  owns draft/attachment confirmation. On 1626, direct document navigation had
  succeeded while the native intent was blocked; reopening the original route
  restored its ready composer and two messages. This does not establish the
  original missing-composer root cause.
- `bab719362`: cached model labels remain immediate UI metadata, but old menu
  handles are no longer submitted after dismissal. A unique fresh semantic
  match binds the pending selection; ambiguous/current-catalog mismatches fail
  closed. Deferred selection no longer dismisses its own pending mutation.
- New-chat readiness: 37 focused JVM cases passed. Model follow-up: 28 cases
  passed after correcting an obsolete test expectation that omitted the already
  shipped file-library preset. XML failures/errors were checked explicitly.
- Normal 1.1.1627 (1627), source `bab719362`, published and installed via `-r`.
  APK SHA-256: `b632fff5c7aa7d29aeca163c1b01598a68bb72918fa8a14bd80a4a1680189d3d`.
- Current-document diagnostics returned complete, one visible prompt, and no
  incomplete flag. The native cached slider selected High, the current official
  catalog confirmed High, and the same native control restored Extreme.
- The native new-chat action produced an empty chat. Header temporary on and
  off each yielded exactly one new `official_temporary_runtime_v1:accepted`
  receipt and matching native/official state. Both cases are scoped completed,
  not coverage of project/work/guest privacy variants.
- Both acceptance scripts restored the original conversation, its two messages,
  empty draft and awake lease. No sends, microphone sessions or uploads ran.
- Native Tools -> Create Image failed: two catalog commands were superseded
  before dispatch (about 200 ms and 6 ms); no selection command ran. A direct
  tracked read then succeeded in 369 ms with `official_tool_runtime_v1:accepted`.
  This isolates a native orchestration failure, not an absent official tool.
  The native coordinator follow-up must be accepted before marking this passed.

Log stems: `document-new-chat-jvm-20260910-083524-487`,
`model-handle-jvm-confirmed-20260910-085514-982`,
`composer-recovery-release-20260910-085828-570`,
`composer-recovery-model-device-20260910-090705-844`,
`composer-recovery-tools-device-20260910-090752-300`,
`composer-recovery-temporary-device-20260910-091422-537`.

Remaining outside this batch: physical drag sampling, other model/tier and
server-preference variants, send after model selection, actual new image
generation, temporary project/work/guest contexts and the other items in the
[remaining matrix](../web-ai-private-native-remaining-batch.md).
