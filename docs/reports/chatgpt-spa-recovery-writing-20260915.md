# SPA Recovery And Current Writing Projection

Status: implementation complete; adapter 418 release and device acceptance pending.
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

The evidence helper has 20 passing positive/negative checks, including changed
content/order/count, invalid scalar types, wrong route, draft and voice activity.
Nine Android tests passed (`conversation-session-unit-release-20260915-094211-919`,
319.1s) and cover confirmed personal/project/streaming route capture without
composer readiness, preview/login/temporary rejection and origin/path safety.
The first invocation used a nonexistent Gradle task; the corrected repository
`:app:testReleaseUnitTest` invocation above passed without failures or skips.
The signed APK release and corrected device recovery case remain pending here.

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
