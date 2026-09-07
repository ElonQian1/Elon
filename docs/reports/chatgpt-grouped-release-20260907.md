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

All three builds used the approved `publish-apk.ps1`, verified the remote artifact,
and completed whitelisted replacement installation on Xiaomi 14 Pro. Native
MCP independently read back each installed version. USB and wireless referred
to the same physical phone; the stale second-device mDNS entry was not used.

| Build | Source | APK SHA-256 |
|---|---|---|
| 1542 | `d5e3e3d829df589bbd6e15394adfb69dac15d944` | `aabb6c12d1934c9e7eb20b58d3ca11f4d704153d49e5386b8eca729bf85ee2b6` |
| 1543 | `ca672ea5cfb1e361a97fe112ea9b1db291d1458c` | `36cfc33d050ccaf99f740df9f763189935be6fd6ee74332e96f64d907b235a2c` |
| 1544 | `54a89232e` | `9354fbb84c9617fb35bcdc2df7999863db044c49d4f8bac6647c99d51cb8a61c` |

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

## Same-origin download follow-up (1544)

The already-retained, SHA-256-verified official shared source identifies the
two exact Estuary content routes and passes resolver URLs to download anchors.
Our external-origin-only handoff had no corresponding binary route. Adapter 293
adds that candidate by reusing the library byte/save owner, with cookies kept
in the page, redirects rejected, selected-file authorization and continuous
account/document/route/cancellation guards. No storage-domain whitelist was
expanded. See [the exact source boundary](../chatgpt-private-file-download.md#same-origin-content-candidate-adapter-293).

Six new targeted tests plus existing related suites pass 118 Node runner cases
and the file-index script's 17 checks. The positive cases failed against the
old implementation first. Release Kotlin/Java compilation and assemble passed
in 6m 43s; this is not a repeat of the earlier 1,007-test Android run.
The release publisher verified remote size/hash and replacement installation
at 1544. MCP independently confirmed `1.1.1544`, running=true.

Wireless ADB briefly went offline and subsequent MCP bootstrap reads timed out.
After reconnecting only the pinned transport and opening the existing main
activity, MCP recovered. The readiness check then reported the phone locked,
so no fixture navigation, upload, send, microphone or download was attempted
on 1544. User unlock is requested. The 1543 rejected URL was never captured;
source evidence for the added route does not prove it caused that failure.
Actual saved bytes and their digest are still required before claiming a fix.

## Resumed 1544 saved-byte acceptance

After the user's retry request, wireless ADB was already online and confirmed the
same Xiaomi hardware. MCP independently returned `1.1.1544`; no installation or
data reset was needed. The handset was initially locked, then a later readiness
check confirmed it unlocked. The baseline was `conversation_home`.

The native social ChatGPT surface became ready with adapter 293, an empty draft
and no pending attachment. The existing synthetic conversation was opened and
its private file index returned one fresh descriptor. One production MCP download
completed successfully: `download_saved`, native state `saved`, 78 received bytes,
and 1,960 ms from command start to terminal receipt. A filesystem check restricted
to the synthetic fixture name found exactly one saved file in Downloads:

- Bytes: `78`.
- SHA-256: `75e2ed9bfe5772c9918e552ed07c2c0e689e7039367c81bb6906c63e396fa1f3`.
- Digest matches the original uploaded fixture.
- The original blank chat was restored, then `conversation_home`; draft length
  and pending attachment count were both zero.

Case `android_chatgpt_private_file_download_v1:ordinary_saved_bytes` is completed.
Reuse this evidence; do not repeat its upload or download without a regression.
This is a production-handler and actual-file acceptance, not rendered native-menu,
multi-file, project/library or gallery acceptance. No new message, microphone,
login, Cookie/data clear or proxy-core change occurred.

The separate image exporter v4 follow-up is source-only and not in this installed
APK; see [its implementation and offline evidence](../chatgpt-private-image-gallery.md#same-origin-preview-source-follow-up).

## Next boundary

The ordinary saved-byte check above passed; inspect any new scoped download-source
rejection before changing another policy. Do not repeat the successful ordinary
upload/download as a substitute for untested scopes. Grouped image/PDF,
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
- `chatgpt-content-download-regression-pass-20260907-183106-269`
- `chatgpt-content-download-publish-20260907-183325-187`

Each has structured result/state plus stdout/stderr files. The bounded
`acceptance-1543.json` and resumable synthetic attachment checkpoint are in the
same research-artifact directory as the JUnit archive. The latter contains only
local test routing and no conversation contents; do not print its route values.
