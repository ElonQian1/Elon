# Cold composer owner preparation

Date: 2026-09-15. Source-batch regression fix, not new device acceptance.
Context module 25; adapter 424. Reuses the existing private sender and loader.

## Failure and Scope

The committed conversation/controller and account could already exist while the
reviewed composer module import was pending. `capturePrivateConversation` starts
or joins that import, but returns no draft synchronously. Context capture then
returned `context_unavailable` before its existing module-loading await. A later
click could succeed even though the first operation never resumed.

This concerns an unmounted composer with an authenticated, committed memory
owner. It does not establish first-send support without page identity, an
observed runtime, or an official client conversation. Existing accepted sender
scopes and candidate flags are unchanged.

## Correction

- Pin the existing committed owner before awaiting the already-observed composer
  module. Reuse the binding loader's single-flight import, deadline and cooldown.
- After loading, recapture the draft and compare document, profile, account,
  route, conversation, controller, shared store, file store and thread mode.
  A replaced owner fails instead of receiving the original command.
- Continue the same operation only with that confirmed owner. Keep the existing
  expected-draft, model/tool/file, transaction and request ownership checks.
- Do not mount/focus the official input, call DOM submit, guess an import, retry a
  POST, or change voice, subtitles, Cookie storage or the independent proxy.

## Evidence

`scripts/test-chatgpt-web-fresh-text-cold-owner.js` uses the real context,
committed-owner runtime, private input and sender modules with synthetic fixtures.
The old implementation rejected while the import was pending; the fixed version
passes all 14 tests. They cover one in-flight import, one readiness notification,
owner/account/document/profile changes, unknown modules, import failure, and a
full sender transaction with zero writes during loading and one POST afterward.

The focused regression run passed 345 tests across this new suite, context,
new/temporary/project sends, transactions, private input, attachment input and
runtime submission. Logs remain under Git metadata:
`fresh-cold-owner-green-20260915-150242-977` and
`fresh-cold-owner-regression-20260915-150339-433`.

No new APK build/install or device latency claim belongs to this fix. Include it
in the next grouped release and verify one cold-module native send there. The
installed 1762 search-caption fix still has its separate live acceptance pending;
this work does not replace that test or the pending process-recreation checks.
