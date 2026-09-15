# Cold composer owner preparation

Date: 2026-09-15. Source-batch regression fix, not new device acceptance.
Delivery update: included in grouped normal Release 1765 / adapter 428, published
and replacement-installed. Production native readiness passed; a cold-module Send
and latency improvement remain unaccepted. See the
[grouped release receipt](chatgpt-runtime-bindings-20260915.md).
Original correction: context 25 / adapter 424. The identity-cache follow-up below
uses context 26, private input 8, transaction 36 and adapter 427. Both reuse the
existing private sender and loader; their device acceptance is separate.

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

The original source batch did not build/install an APK or claim device latency.
The subsequent grouped release above includes it; verify one cold-module native
send there. The separate search-caption fix subsequently passed one supervised
native sample on 1765; that does not accept the pending process-recreation checks.

## Cold Identity Cache Follow-Up

The native input observer required `context.stamp()` before calling context
preparation. That stamp requires the loaded shared module's account state, but
the observer did not initiate that module's load. On an existing conversation
with no composer DOM, it could wait for an unrelated feature to warm the cache.
The new regression reproduced zero imports across 20 input snapshots. This is
a local observer deadlock, not proof of an account/login or network failure.

The observer now imports the already-observed shared module through the existing
versioned loader when the account stamp is not available. This imports code only:
it captures no user command, edits no draft and issues no preparation/send POST.
It does not guess asset names or synthesize page/account/conversation state.
An account becoming available schedules fresh normal input preparation, which
still verifies the committed owner, draft, model, files, tools and parent.

There is one local in-flight observer, a five-second deadline, ten-second failure
cooldown and at most three attempts for the same document/route/profile/scope.
The module loader retains its own shared import and deadline. Hidden/offline
pages start no new work; network flapping cannot erase the cooldown. A changed
document, token, route, profile or binding instance invalidates late notification.
An account change while importing cannot inherit draft authority: no draft has
been captured, and the next snapshot must validate the current owner anew.
Loaded-but-signed-out state does not become ready and is not repeatedly imported.

The same regression found that a missing model could be advertised as ready by
the input observer even though request construction would later reject it. Context
admission now uses the request's model-ID format before publishing readiness.
Missing/malformed model data is `context_unavailable / base_model`, not a claim
that the official model or capability does not exist. Arrival of a valid model
allows the existing bounded input recheck to succeed without a reload.

The new test composes the actual private input, fresh context and committed-owner
runtime. Fixtures cover cold shared and editor modules, reentrant notification,
failed imports, stale/disabled/background/offline state, account changes, absent
owners, uploads and delayed models. Existing independent-send and retry tests
remain in the batch. The initial combined run also caught an incorrect fixture
assertion counting the pre-existing constructor's composer prewarm; the fixture
now excludes that constructor baseline rather than changing production prewarm.

`cold-identity-final-20260915-163059-642` passed 600 Node tests, zero failures,
errors, cancellations or skips, in 5.2 seconds. Coverage includes the new cold
identity cases, fresh sends/retries, private input, runtime bindings, full asset
assembly and pending-recovery wiring. This does not add 600 live provider samples.
No separate Android build or live latency acceptance belonged to this source
follow-up; the subsequent grouped release is recorded above. Accepted private-send
defaults are unchanged.
This closes cold shared-module preparation, not a completely uninitialized page
without authenticated identity, a committed conversation or a reviewed runtime.
