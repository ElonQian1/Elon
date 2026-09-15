# SPA Recovery And Current Writing Projection

Status: normal 1756 / adapter 419 published; route recovery verified, full-body
recovery not yet accepted. Sparse snapshot regression fix is under verification.
This extends the [September 15 compatibility batch](chatgpt-runtime-bindings-20260915.md),
not another private sender or Canvas implementation.

## Current Writing Projection

The canonical runtime loader already supports both reviewed September 15
profiles. `chatgpt_web_text_blocks.runtimeProjection` still admitted only the
September 12 profile, so current in-memory Writing Block content was skipped
on the DOM-message projection path after the official update. History/private
stream projection has its own existing path and was not replaced.

Parser 10 admits exactly `web_20260912`, `web_20260915` and `web_20260915_b`.
It reuses the loaded canonical shared namespace. Rendering does not import,
fetch, mutate state or wait for the composer. Unknown profiles, absent identity,
ambiguous owners, unfinished messages and foreign projects remain rejected.

- The new tests reproduced six missing current-profile projections before the fix.
- `writing-runtime-current-regression-20260915-092437-349`: 37 passed, no skips
  or failures, including the real resolver and both exact reviewed export maps.
- This is Writing Blocks, not proof of original Canvas editing or saving.

## Process Recreation Gap

`conversation-process-recovery-diagnosis-1754-20260915-093653-706` exercised a
completed owned project fixture on normal 1754 / adapter 417, with zero Send
clicks. The app process stopped and was recreated, but both native and official
routes differed from the expected route. Original native/web body counts were
18/17; the reopened route had 2/2. Manual original-route and awake restoration
succeeded. This is a reproduced navigation failure, not a body-hash-only failure.

The background session persisted body snapshots after SPA navigation but saved
the startup URL only through `onPageFinished`. That callback is not emitted for
every in-document route change. Thus a restart could display cached content and
then load a different, previously saved route.

`ChatGptWebSessionRestorer.onSnapshot` now records a confirmed authenticated
conversation URL from the production snapshot path, independently of composer
readiness. Cached content-only previews, login-required states, temporary chats,
transient home/project pages and unsafe origins cannot overwrite that pointer.
The existing private preferences store skips duplicate URL writes. No new
message persistence, polling, network request or credential store is introduced.

## Acceptance Contract

`smoke-chatgpt-conversation-process-recovery.ps1` selects only the external
resolved owned fixture, refuses pending writes or active capture, records idle
native/official message fingerprints in memory, stops only `com.elon.app`, and
reopens only the native product surface. It must not navigate to the expected
conversation to manufacture a pass. It requires a new process and exact route,
body ordering/counts, idle state and zero new send attempts, then restores the
original route and awake settings. Receipts contain only booleans and counts.

The evidence helper initially had 20 passing positive/negative checks, including changed
content/order/count, invalid scalar types, wrong route, draft and voice activity.
Nine Android tests passed (`conversation-session-unit-release-20260915-094211-919`,
319.1s) and cover confirmed personal/project/streaming route capture without
composer readiness, preview/login/temporary rejection and origin/path safety.
The first invocation used a nonexistent Gradle task; the corrected repository
`:app:testReleaseUnitTest` invocation above passed without failures or skips.
Normal 1755 from `f076b17a0ed868ffd4880f58dfbb7c6962f1b274` passed release build,
remote hash/size verification and unattended Xiaomi installation in 446s.
SHA-256: `4c21caead57f449a4221c38302f10a275a102c1db5692d45029561e312e5f4a5`.

`conversation-process-recovery-1755-20260915-095844-044` restored both correct
routes but body counts changed 23/23 to 9/9, so it failed. A shorter diagnostic
matched 15 native / 14 web messages after restart in 10.666s, but its web window
started at 4 with 18 observed messages and `context_complete=false`. That is
only window continuity, not complete history acceptance, and is not promoted.
Both restored the original route/awake setting with zero sends. A subsequent
explicit private read exposed 18 complete messages, with stable UUID identities.

## Startup Read Follow-Up

Complete private history reads were wired to explicit conversation navigation,
not process startup. Adapter 419 reuses the existing current-conversation read
once after the restored route has confirmed private runtime readiness. It does
not require usable composer DOM, poll, reload the page or replay a send. Active
streaming/dictation delays this read; a confirmed different conversation or clear
cancels it. Failures retain the existing transport policy and manual refresh.

`ChatGptStartupHistoryRefresh` owns this small one-shot decision separately from
the background session. Unit cases cover delayed identity, content-only previews,
project ownership, navigation, capture and repeated snapshots. The full-body
acceptance now requires a complete zero-offset web history, no export truncation,
and equal native/web counts before comparing exact per-surface fingerprints.
The evidence helper passes 26 cases; partial windows cannot manufacture a pass.
`startup-history-unit-20260915-101334-208` passed in 313s: 13 Android tests,
zero failures/errors/skips, including the new one-shot recovery cases. Adapter
419 normal release 1756 passed in 453.2s and was installed without a data reset.
Source: `3d2114ec4`. SHA-256:
`6c927b9e0730d6fa97ebd76ec3dd20c1bed9bd1c5d2af5892584e7fd6d2e3088`.

## Sparse Snapshot Regression

Two 1756 checks stopped before process termination because the strengthened
baseline never became complete. The diagnostic run
`conversation-baseline-diagnosis-1756-20260915-103407-696` reported 14 native
rows / 13 web messages, window start 5, observed count 18 and incomplete context.
The extra native row is the existing missing-history notice. Both runs sent
zero messages and restored the original route and awake setting.

The merger replaced the entire span between first/last matching IDs with a
sparse incoming DOM subset, silently discarding unmatched messages in between.
The new 18-message/three-known-row regression failed against the old code
(`sparse-history-red-20260915-103819-422`, 113.1s). This explains how a complete
private read could shrink again after the next partial snapshot.

Adapter 420 preserves missing rows when the incoming ordered subset contains
only already-known exact message identities. It still updates the observed
rows; it does not deduplicate by text. A complete private content-only history
read is separately authoritative and can replace an old branch or inflated
window. Active streaming and partial/empty reads cannot claim that authority.
The existing 80-message bound and cross-conversation isolation remain enforced.
`sparse-history-regression-20260915-104145-106` passed 31 Android tests in 308.7s.
After moving the unchanged conversation ownership check out of the background
entry into the merger, `sparse-history-ownership-regression-20260915-104758-178`
passed 32 tests in 377.8s with zero failures/errors/skips. The source-size and
document guards passed. Adapter 420 release and full-history device evidence
remain pending.

## Boundaries

- Idle completed-conversation process recreation is not an in-flight unknown
  write recovery test. The existing send ledger is in memory; this change does
  not make it durable, replay a request or claim such recovery is complete.
- Real network-loss acceptance remains pending. The handset currently has only
  wireless ADB; Wi-Fi/VPN were not disabled to manufacture a network incident.
- Original Canvas edit/save/conflict/history/restore is implemented and covered
  offline but still lacks an eligible real owned original Canvas fixture.
- Cold first send with no initialized page identity/runtime remains distinct
  from the already verified no-usable-composer first send.
- Existing verified audio/subtitle/dictation/read-aloud, Cookie/login state,
  independent proxy and default private-send scopes are unchanged.
