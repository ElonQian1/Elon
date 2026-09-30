# Compact Covers And PWA WeChat Handoff

## Scope And Ownership

- Shared Web/PWA/Win card geometry stays in `social_links.js` / `social_links.css`.
- Android uses the equivalent pure `SocialPosterSize` policy; the existing card and opening bar remain in place.
- Public thumbnail encoding stays in the existing server `link_previews/cover.rs` module.
- PWA external navigation is isolated in `social_wechat_handoff.js`, mounted only by the mobile chat view. Native Win handoff and Android routing are unchanged.
- No new private playback endpoint, account transfer, arbitrary Intent forwarding, or WeChat callback protocol is invented.

## Root Causes And Changes

1. Server posters were reduced to 256 pixels on the longest edge, then displayed much larger in physical pixels on high-density phones. Posters now retain up to 640 pixels, still within the existing 96 KiB inline limit. Detailed images progressively shrink rather than disappearing. Small source images and avatar limits are not enlarged.
2. Android cards always occupied up to 280 dp. Shared Channels cards forced 3:4 regardless of the source. Loaded posters now use the real ratio (bounded 1:2 to 2:1), with portrait width up to 208, square 220, landscape 280, and poster height up to approximately 280 CSS px/dp. Old low-resolution covers get narrower cards. Tiny raster sources are contained, not stretched. Text and open-mode controls remain separate from the raster.
3. PWA Channels clicks previously opened an external browser tab, then depended on the website to open WeChat. The new path requests the existing authenticated, short-lived handoff directly from the same PWA document. Android uses a package-scoped Intent; iPhone/iPad use the validated `weixin://biz/finder/openFinderFeed/` scheme. It does not create a new tab, change the HTTP URL, clear chat state, or invent a return URL.
4. If asynchronous preparation consumes browser user activation, a visible `在微信打开` button obtains a fresh tap. Requests are single-flight, cancellable, bounded to 12 seconds, and discarded on hide/dispose/account or group changes. Signed scenes remain in memory only. Failure allows explicit retry/copy, without silently navigating the PWA away.

## Verification

- Node production-module tests: URL policy/cards, geometry, scene validation, cancellation, stale responses, retry, duplicate click, and listener cleanup passed.
- Playwright: six cover dimensions at 360px/DPR3 and 1280px/DPR1 passed bounded layout, ratio, tiny-image and overflow checks. Production shared components, not a separate mock layout.
- Public Channels sample `Aur6t4pfk3`: actual cover/avatar rendered at desktop/mobile widths; keyboard, click and fallback checks passed. Public Bilibili plus Douyin/Xiaohongshu fallback layout checks passed.
- Chromium/Android-emulation and WebKit/iPhone-emulation user-gesture checks: no additional tab, unchanged document URL, unsent draft and scroll position after the external scheme request. These are actual browser-engine checks, not real iOS or external app playback/Back-stack simulation.
- Rust: both targeted cover tests passed, including detailed-image budget reduction and no upscaling.
- Android: 18 targeted tests passed (card interactions, media opening, sizing), zero failures/errors.
- Registered Xiaomi reconnected by project registry and verified hardware identity. Its installed WebAPK belongs to an unrelated website, not Yilong; do not alter it or count it as Yilong PWA acceptance. Honor was offline.

## Explicit Limits

- PWA code can preserve its document and remove the intermediate browser tab. It cannot force WeChat's Back behavior, receive a made-up video-ended callback, or restore a document already killed by the OS.
- The user confirmed the reported return problem is on iPhone. iOS-specific handoff has been added and covered by unit tests, but real iPhone installed-PWA -> WeChat video -> Back acceptance remains pending. Android testing must not substitute for this result. Desktop retains its native path.
- Embedded Channels playback remains unsupported; thumbnail availability does not mean an authorized video source exists.
- Release and registered-device installation results are recorded below; browser-engine checks do not replace physical iPhone acceptance.

## iPhone Evidence And Boundary

- Re-read the live public frontend on 2026-09-30: `https://res.wx.qq.com/t/wx_fed/finder/web/finder-preview/res/assets/mmfinderopenwebapisvr.dd823d0e.js` detects iPhone/iPad as mobile, builds `weixin://biz/finder/openFinderFeed/` with a single URI-encoded scene, then clicks an anchor. It contains no usable third-party return callback in that flow. This supports the launch scheme, not a claim that Back returns to our PWA.
- [Apple WWDC23 web apps documentation](https://developer.apple.com/videos/play/wwdc2023/10120/) describes out-of-scope links opening in Safari View Controller on iOS Home Screen web apps. Avoiding the intermediate HTTPS page addresses our extra navigation layer, but WeChat/iOS still own external-app return behavior.
- No spoofed browser identity, fabricated `return_url`, history trap, timers that relaunch the app, or notification-based return workaround was introduced.

## Release Evidence

- Android `1.1.1838 (1838)` published from `00ed7aa68701b058927c15104368434a606fee9e`, which contains both feature commits. The publisher fast-forwarded over unrelated macOS workflow/test changes before building. APK SHA-256: `7e98c53f22b26b40fd2cb4f44812e8bbc568bf42a1bc18fbfd8845e177b375d8`.
- Release source-asset and manifest checks passed. Registry-driven postflight installed with data preserved and read back build 1838 on the Xiaomi; Honor was offline. `check-task-complete.ps1 -Kind AndroidFeature` passed with exact APK source provenance before this documentation update.
- Server `0.3.1802` and its PC bundle published from `706f0137732bda8ada28d9a7720857f2ed8ed792`; health and release markers matched. All three individual media asset endpoints matched source (normalized line endings), with `no-cache`. Server and PC input diffs between this source and APK source `00ed7aa` are empty; the only intervening paths are a macOS workflow and a desktop Rust test. The generic Server/PcFrontend completion checks reject this unrelated HEAD advancement, so their strict SHA check did not pass; no duplicate server rebuild was used to hide this distinction.
- Live-page verification caught a separate stale runtime template: the asset endpoint was current, but the PWA homepage still embedded the Android-only handoff generation. `publish-mobile-pwa-static.ps1` atomically published the complete source `00ed7aa` generation, hash `79999b6168028096cab3a81bcbf9fa8287ef625d4f2227301a064f5dcf15f5f3`.
- Added `scripts/test-live-pwa-media-assets.mjs` to parse the served page without executing its application code. Card JS, CSS and the iOS-capable handoff module now match the actual inline runtime on both `http://43.139.149.158:8080/web` and `https://43.139.149.158:8443/web`. Checking only separate asset URLs is insufficient when the runtime template is active.
- No physical iPhone was available: installed PWA -> exact WeChat video -> return to the same PWA remains explicitly unverified. The user was asked to confirm the destination and return behavior after reopening the PWA. No login state was cleared.
- Final integration boundary: rebasing the evidence-only commit incorporated the unrelated project-introduction feature `f5ffcbe48`. The unified `AndroidFeature` finish check then reported `FINALIZABLE=false` because published APK 1838 does not include those six newly merged Android files. This batch's earlier exact-source APK acceptance remains valid; the latest whole-main release is not claimed complete. Task worktree is clean, main was fast-forwarded, and no untracked baseline files were reported. Do not weaken the completion kind or rebuild unrelated work merely to relabel this receipt.
