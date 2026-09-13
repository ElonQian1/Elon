# Grouped ChatGPT Recovery Release: 1716

Date: 2026-09-14. Status: `published / package_verified / device_deferred`.
This is delivery evidence, not a new provider or native UI acceptance pass.

## Included Source

- Source: `50d384efd0339b8125bd8d1494c74a3e3377c3c8`.
- Production delta from 1715: recovery coordinator v2, text transaction v22
  and adapter 386 from `b83f17844c0c86b6e9095e9fa1f664b5f86b73c2`.
  A foreground/online event during cooldown or an in-flight read is retained
  for one bounded read-only recovery. No text replay or WebView reload is added.
- The same source contains exact observed-route handoff for first-send
  acceptance. This is test tooling, not permission to replay pending writes.
- Writing Blocks, DOCX, linked-library hydration/save and other ChatGPT inputs
  remain identical to the [verified 1715 package](chatgpt-private-grouped-1715.md).
  Do not recreate them or repeat accepted ordinary editor/export cases.

## Build and Download

- `web-ai-recovery-grouped-release-20260914-031341-549` passed in 540.3 seconds,
  including Release Kotlin/Java compilation, vital lint, packaging, server
  publication, remote verification and the bounded ADB deployment attempt.
- Version `1.1.1716`, code `1716`, package `com.elon.app`.
- APK size: 40,441,877 bytes.
- SHA-256: `e2efbed7642a29332982ed0fc9d0564676b4aa387106995ce8987cdc89ffa434`.
- Independently read `/app/version.json`; source, version, size and hash match.
- Downloaded the actual `/app/ElonSpeed-latest.apk` and verified its full hash,
  aapt package/version and all 154 `assets/chatgpt_web*.js` against source bytes.
  Missing assets: 0. Mismatched assets: 0.
- The initial download hit its 100-second timeout after 35,068,848 bytes.
  After rechecking the release alias, a range resume completed in 15 seconds
  (`web-ai-recovery-apk-download-resume-20260914-032451-001`). The full-file hash,
  not the range result alone, establishes artifact integrity.

The exact APK is retained under
`$GIT_COMMON_DIR/ai-acceptance-artifacts/chatgpt-grouped-1716/elon-1716.apk`.
Use this artifact if the latest alias advances; no rebuild is needed to resume
the same acceptance round.

## Reused Test Evidence

The 185 passing recovery/transaction tests in
[the source report](../chatgpt-fresh-recovery-wakeup.md), 205 linked-writing Node
tests and 58 Android tests in the 1715 report are reused. They were not rerun or
counted as new tests this round. The only changed Kotlin input is adapter 386;
the complete Release build above verifies its inclusion.

## Device Boundary

USB ADB listed no device and mDNS discovered no wireless service. The publisher's
existing paired Xiaomi target also timed out; final installation result is
`APK_ADB_DEPLOY_STATUS=verification_deferred`, not installed or accepted.
No user conversation, writing save, microphone, Cookie/application data or proxy
configuration was modified. The unresolved 1709 first-send fixture is untouched.

Next, use its exact read-only resolution before any new first-send trial. Then
verify native first-send/follow-up, one pending-turn network/foreground return,
and the outstanding owned project/typed/linked-writing save and mobile DOCX
cases. Unknown provider scope remains unsupported, not fabricated from this
package pass. First-send variants are still trial-only; Google remains last.
