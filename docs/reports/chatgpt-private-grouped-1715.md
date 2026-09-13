# Grouped ChatGPT APK Verification: 1715

Date: 2026-09-14. Status: `published / android_verified / device_deferred`.
This is package and offline verification, not provider/device acceptance.

## Source and Android Tests

- Tested source: `01f69883c1578252278c01e8e62e3cd40c10a8cd`.
- `web-ai-grouped-android-tests-20260914-022318-541`: Release production and
  unit-test Kotlin/Java compilation passed; 58 tests across nine suites passed,
  zero failures, errors or skips, 325.3 seconds including compilation.
- Tests cover text block parsing/inventory/continuity, native UI contracts,
  edit history, DOCX export, writing-save arguments, fresh-send trial wire and
  private input readiness. They do not simulate a real logged-in provider.
- Existing 205-test linked-writing regression and 267-test first-event route
  regression are reused from their recorded source batches, not rerun or
  represented as additional tests executed in this package round.

## Published Artifact

`web-ai-grouped-release-20260914-022910-858` returned `already_covered`:
a concurrent mainline release had already published the required Android inputs.
This task did not claim another version or rebuild the same package.

- Version: `1.1.1715`, version code `1715`, package `com.elon.app`.
- Published source: `26676dd2230d53eabc735e676414c8b9c3a20767`.
- The only source commit after the tested baseline changes OKX modules/tests,
  not ChatGPT. Git ancestry and scoped Android paths were checked.
- Size: 40,441,357 bytes.
- SHA-256: `ca25d30c2553a4b8fe054e08601153181737d1f726e0bc54710192037610e7f6`.
- Downloaded the actual `/app/ElonSpeed-latest.apk`, checked the whole-file hash
  against `/app/version.json`, and verified package/version using Android aapt.
- All 154 `assets/chatgpt_web*.js` entries match current source bytes exactly;
  there are zero missing entries, mismatches or line-ending-only differences.
- This includes adapter 385 wiring, runtime bindings 28, writing context 6,
  library session 2 and the new versioned Library reader 1.

The exact APK and nine JUnit XML results are retained locally under
`$GIT_COMMON_DIR/ai-acceptance-artifacts/chatgpt-grouped-1715/`.
Use that artifact/hash if the latest-download alias advances; do not rebuild
only to resume the same acceptance round.

## Device Gate and Next Actions

One current USB/wireless ADB availability check found no attached device.
No APK was installed, no microphone was opened, no account message or writing
save was sent, and no Cookie/application data or proxy setting was changed.

1. On the same trusted device, resolve the retained 1709 uncertain first-send
   fixture through exact read-only user-message/provider/native matching.
   Its `replay_allowed:false` barrier remains; this package does not resolve it.
2. Accept one native new-conversation first-send/follow-up with current route,
   unique messages, complete reply and original-state restoration. New first
   sends stay trial-only until that succeeds.
3. Accept an explicitly owned library-linked Writing Block: native open/edit,
   versioned preparation, one save, exact readback and reopen. Include the cold
   Library-session path; do not infer it from the already-hydrated path.
4. Continue the existing project/typed-writing and fresh tool/attachment/
   temporary/project scope matrix in this same package. Reuse previously
   accepted ordinary editor/export and native voice/dictation scopes.

Google private sending remains last by user priority. Unresolved protocol
scopes and unperformed device cases are not marked completed by this report.
