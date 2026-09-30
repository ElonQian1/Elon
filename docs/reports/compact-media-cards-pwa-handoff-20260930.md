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
- Release and registered-device installation results are recorded separately after publishing.

## iPhone Evidence And Boundary

- Re-read the live public frontend on 2026-09-30: `https://res.wx.qq.com/t/wx_fed/finder/web/finder-preview/res/assets/mmfinderopenwebapisvr.dd823d0e.js` detects iPhone/iPad as mobile, builds `weixin://biz/finder/openFinderFeed/` with a single URI-encoded scene, then clicks an anchor. It contains no usable third-party return callback in that flow. This supports the launch scheme, not a claim that Back returns to our PWA.
- [Apple WWDC23 web apps documentation](https://developer.apple.com/videos/play/wwdc2023/10120/) describes out-of-scope links opening in Safari View Controller on iOS Home Screen web apps. Avoiding the intermediate HTTPS page addresses our extra navigation layer, but WeChat/iOS still own external-app return behavior.
- No spoofed browser identity, fabricated `return_url`, history trap, timers that relaunch the app, or notification-based return workaround was introduced.
