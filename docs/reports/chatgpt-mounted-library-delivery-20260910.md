# Mounted library grouped delivery

## Artifact

Normal Release **1.1.1631 (1631)**, source `feb9992ac`, groups mounted catalogue
download (`57435cf2d`) and mounted attachment preparation (`8ed89f7e4`).
The unrelated old attachment-lease assertions were corrected separately in
`eb3e2e792`; production dispatch/ACK cleanup was not changed by that test commit.

APK SHA-256:
`665337b502ade62434a96a5de57a6186b429a5d14e6c53eaaa21122812063c62`.
Standard published endpoint:
`http://43.139.149.158:8080/app/ElonSpeed-latest.apk`.
It is mutable; verify the version/source/hash before reusing this endpoint.

Logged release `mounted-library-grouped-release-20260910-131441-971` completed
in 488.2 seconds. Gradle reported `BUILD SUCCESSFUL in 6m 34s`; the publisher
verified remote APK SHA-256/size and reported `APK_RELEASE_STATUS=published`.
Both new capabilities remain **not device verified**, not `completed`.
The related attachment set passed 196/196 and the prior mounted download set
244/244; 109 assembled production JS assets parsed successfully.

## Device boundary

Before this batch, wireless `192.168.31.171:5555` executed real commands and
matched the pinned Xiaomi hardware `e0d909c3`. Later, it remained listed as
`device` but individual shell/MCP calls timed out. One subsequent bounded
identity read recovered; that did not prove sustained installation availability.
The inventory showed no USB transport during these checks.

The publisher attempted its configured wireless target three times; every
`adb connect` timed out before installation. Its final markers were
`APK_ADB_DEPLOY_STATUS=verification_deferred` and
`REAL_DEVICE_STATUS=offline_or_unavailable`, after its deferred-verification
handler stopped the local ADB server. The outer logged-command success therefore
means **published with device verification deferred**, not installed or accepted.
Last independent installed-version proof remains 1630; do not infer 1631 from
the online manifest alone. No app data/Cookies or proxy settings were cleared.

A final bounded recheck restarted ADB through `devices -l`: the inventory was
empty (no USB or wireless transport), and `connect 192.168.31.171:5555` timed out
after 12.6 seconds. No subsequent installation or device acceptance was attempted.

One separate stalled readonly `dumpsys power` child was terminated only after
matching its exact PID, parent and command line; that did not stop the ADB server.
The publisher also reported a nonfatal worktree auto-cleanup `Branch`-property
warning. The mandatory task finisher still owns Git/main/worktree closure.

## Resume

Do not rebuild merely because wireless installation failed. Reuse this exact
artifact, pin the physical device, install with `-r`, and verify package version
1631 plus active adapter 314. Continue from the production native chat surface.
No microphone test is required for this batch.

Use an authorized existing cloud test file to accept native Library download
and saved bytes, then native attach/card/remove and one explicit isolated
file-content send with source/backing association. Do not manufacture a cloud
account or label ordinary-file tests as mounted-provider acceptance. Preserve
the original chat/draft and existing library files. If no suitable cloud file
exists, record that precise device-data gap and proceed with the already-pending
native retry, sharing and citation cases in the remaining-work matrix.
