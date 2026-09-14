---
version_status: historical
reviewed_at: 2026-09-14
---

# Private-Native Release Checkpoints 1540-1544

Historical delivery evidence moved from the remaining-work index on September
14. These checkpoints do not describe the current installed APK or current
acceptance gaps. Use the [current status map](../web-ai-private-native-remaining-batch.md#current-status-map)
and individual capability documents before planning more work.

## 1542 Through 1544 Acceptance

The [batch report](chatgpt-grouped-release-20260907.md) records unified
compilation/tests, three replacement installations, actual private attachment
association and file-content reply, the file-index regression/fix, and the
remaining download-source rejection and 1544 same-origin candidate awaiting
an unlocked-phone saved-byte check. At that checkpoint the complete ChatGPT
acceptance gate had not passed and Google remained deferred.

## Earlier 1541 Acceptance

On 2026-09-06 after the handset returned, the grouped Release production and
unit-test compilation passed all **33 tests across seven attachment suites**,
with zero failures, errors or skipped cases. The latest focused Node run passed
96 cases. These are targeted checks, not a full regression or thermal A/B.

`publish-apk.ps1` published `v1.1.1541` (code `1541`) from `ac2f1662f` and verified
the remote APK size and SHA-256:
`15e20f7cda24e0bfc2a9b7c67fb2884141c2d159cddec95463328488f4a0ef4a`.
The whitelisted postflight installed it on Xiaomi 14 Pro using replacement
installation and read back build 1541. Cookies and application data were kept.
Both ChatGPT and Google returned HTTP 200 in the APK network check before the
grouped acceptance. No accelerator configuration or core was changed.

The production social-AI chat successfully staged and removed the fixed text
fixture, then sent it **once**. The native attachment state reached `completed`,
pending count became zero, and the assistant reply contained both the unique
request marker and the fixture's first line, which was not supplied in the
prompt. The initial assertion stopped on a PowerShell closure failing to resolve
its named helper, not an APK upload failure. Capturing the helper scriptblock
fixed that boundary; the contract test now executes the real predicate across
a module boundary. Resuming the persisted `reply_requested` checkpoint verified
the existing reply without dispatching another message. The fixture was removed,
the production acceptance case registered, and the phone returned to its original
conversation-home surface. No microphone was used.

This proves the production file-delivery workflow, **not** which upload route
ran: its private-association receipt was not retained, and the latest command
had already advanced to send/skin state. Do not count it as integrated private
upload, image, PDF, project or saved-download acceptance. Collect existing
semantic receipts during the next scoped check; do not repeat protocol research
or rebuild the unchanged APK merely to recover that missing evidence.

## Earlier 1540 Checkpoint

On 2026-09-06, `publish-apk.ps1` built and published `v1.1.1540` (code `1540`)
from `ccc76ed37e31364f02c03af333a13a63b30c4bdf`. Remote version, size and SHA-256
were verified. APK SHA-256:
`ef29913013d10a170e16a1ce7d8a2648377495edabeb3f0c6fb62c26eb67755c`.
The standard whitelisted-device postflight used `adb install -r` and read back
build `1540` on Xiaomi 14 Pro. Cookies and application data were preserved.

Installation is not production UI or protocol acceptance. MCP health initially
responded after the update, but later health calls timed out and a plain ADB
process query returned `error: closed`. Both existing command helpers experienced
failures at different times, so there is no confirmed helper-specific defect.
No protocol-capture lease, synthetic upload, new message, or microphone test was
started in that initial installation round. Browser navigation also timed out;
it supplied no protocol evidence.

The resumed round reconnected the same handset. Its accelerator `1.0.139 (140)`
crashed on a missing JNI restore method; the accelerator owner fixed and
installed `1.0.140 (141)` without this task changing proxy code or settings.
After network recovery, one new synthetic attachment attempt through production
`send_input` completed and produced a native streaming acknowledgement. The
capture observed HTTP 200 reservation JSON and official conversation SSE. It
did not establish the complete upload/finalize protocol or independent private
dispatch. The probe was cleared and the UI restored to conversation home with
an empty draft and no pending attachment; details and limits are in the
[recovered-network capture](../chatgpt-private-protocol-evidence.md#recovered-network-capture).

The candidates implemented the narrow prepare/upload/finalize and
composer-association contract and are included in 1541 above. The checkpoint
called for remaining route-specific acceptance with bounded production MCP
commands, not implementing the same transport again.
The [reservation regression](../chatgpt-private-protocol-evidence.md#reservation-completion-regression)
invalidates generic HTTP completion as upload proof. Its correction was required
before accepting attachment delivery, with current draft, conversation and voice
state preserved. A debugging connection failure did not justify another probe
framework, guessed endpoint or repeated application restart.
