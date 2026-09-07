# ChatGPT grouped release and device checkpoint

Evidence date: 2026-09-07. This report records results, not a completed Goal.
The current work list is [remaining private-native work](../web-ai-private-native-remaining-batch.md).

## Unified compilation and tests

The production callback type mismatch was already fixed in upstream `e9c3ff470`.
The clean task branch fast-forwarded and reused that fix; no duplicate production
patch was introduced. Ten legacy test files still asserted old module locations,
the old model/tool sheet, narrower background guards or submit-only dictation.
Commits `04ad6239c` and `d5e3e3d8` updated those contracts to the current owners,
including cross-module wiring, without deleting or skipping cases.

Release production and test sources compiled. The final run passed **209 suites,
1,007 tests, zero failures/errors/skipped cases**. This is the selected WebChat
and `chatgptweb` unit-test set, not every Android test or every live protocol.

Raw JUnit evidence is retained outside the worktree at
`<git-common-dir>/ai-research-artifacts/chatgpt-grouped-20260907/junit-20260907-172412.zip`.
SHA-256: `3693afa18b7f0061a969ca3e6e4eff1a9de1903f582514fbfb7b4172edaf4297`.

## Published builds

Both builds used the approved `publish-apk.ps1`, verified the remote artifact,
and completed whitelisted replacement installation on Xiaomi 14 Pro. Native
MCP independently read back each installed version. USB and wireless referred
to the same physical phone; the stale second-device mDNS entry was not used.

| Build | Source | APK SHA-256 |
|---|---|---|
| 1542 | `d5e3e3d829df589bbd6e15394adfb69dac15d944` | `aabb6c12d1934c9e7eb20b58d3ca11f4d704153d49e5386b8eca729bf85ee2b6` |
| 1543 | `ca672ea5cfb1e361a97fe112ea9b1db291d1458c` | `36cfc33d050ccaf99f740df9f763189935be6fd6ee74332e96f64d907b235a2c` |

1543 is a correction found during the grouped phone round, not a rebuild per
unrelated feature. Its Release build passed; the 1,007-test result above belongs
to 1542, not a second full rerun on 1543.

## Production acceptance

- The actual social-AI chat, not the old test activity, reported authenticated,
  composer-ready, adapter 291 and 146 conversations / 13 projects on 1542.
- The fixed 78-byte ASCII file was staged, removed, staged again and sent exactly
  once through the native production composer. The assistant returned the unique
  marker and the fixture's first line, which was not included in the prompt.
- The current `request_attachment_upload` receipt was successful with
  `private_attachment_associated`. Commit `60c8d3cfe` retains that evidence before
  restoring context and rejects old/missing/fallback/failed receipts. Its executed
  PowerShell contract and the verification-evidence contract passed.
- Pending attachment count returned to zero and the local fixture was removed.
  The original conversation restore assertion passed. No microphone was started.
- On 1542 the same conversation's file-index command returned `files_not_ready`.
  Code inspection reproduced a specific defect: explicit reads inherited the
  background policy's two-minute official-response freshness requirement.
- `ca672ea5c` separates explicit file reads from that freshness test, retaining
  identity availability, the enabled switch, cooldown, deadlines and GET
  coalescing. The expiry test failed before the fix and passed afterward;
  17 file-index cases and the combined 33 Node runner tests passed.
- On 1543 an initial read issued during page loading failed; it was not counted
  as accepted. After ready, the existing fixture's index returned
  `private_files_ready` in **716 ms**, one text descriptor and a download handle.
  A targeted repeat more than five minutes after that read also succeeded in
  **1,507 ms**, with one descriptor and no draft or pending attachment.
- One download attempt then returned **`download_source_unsupported`** with
  native stage `failed` and zero received bytes. No matching synthetic file was
  saved in Downloads. Authorization passed the code's success checks, but its
  returned source did not satisfy the current URL policy. The actual URL shape
  was not captured; do not guess its host, broaden the allowlist, replay a signed
  URL or claim completed downloads from this result.

This is a production-handler MCP check, not rendered picker/menu acceptance.
The original WebView identity, Cookie, login state and independent proxy were
preserved. No credentials or private conversation text are included here.
The handset was restored to `conversation_home` after the bounded checks.

## Next boundary

Inspect the actual download resolver/source shape, then complete the supported
byte route using the existing download owner. Reuse the already-uploaded fixture;
do not repeat its successful upload merely to test downloads. Grouped image/PDF,
multi-file/project attachment, gallery, share and runtime generation checks are
still pending where the work list says so. Do not mark all ChatGPT work complete,
revisit proven voice/dictation transports or start Google ahead of this gate.

## Evidence locations

Logs are under `<git-common-dir>/ai-command-logs/`, with these stem names:

- `chatgpt-grouped-contract-final-20260907-172412-900`
- `chatgpt-grouped-apk-publish-20260907-172920-858`
- `chatgpt-1542-production-readonly-20260907-173833-214`
- `chatgpt-1542-attachment-send-20260907-174548-975`
- `chatgpt-explicit-files-regression-20260907-175459-602`
- `chatgpt-explicit-file-read-publish-20260907-175832-019`

Each has structured result/state plus stdout/stderr files. The bounded
`acceptance-1543.json` and resumable synthetic attachment checkpoint are in the
same research-artifact directory as the JUnit archive. The latter contains only
local test routing and no conversation contents; do not print its route values.
