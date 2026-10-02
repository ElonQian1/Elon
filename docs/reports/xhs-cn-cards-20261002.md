---
version_status: report
reviewed_at: 2026-10-02
---

# Xiaohongshu cn Share Cards

## Scope And Cause

- The reported `xhslink.cn/o/7QvqZwsx2Ch` and `xhslink.cn/o/4Oatsa1J8RD`
  were classified as generic pages because server, PC/PWA and Android only knew the older `.com` short domain.
- Credential-free desktop requests redirected to a login shell. Normal mobile Safari requests returned
  the public, same-note hydration payload, including the cover and `user.nickName`.
- The new share wrappers have no bracketed title. Their original message text must not be silently hidden.

## Delivered Changes

- Exact `.cn` short-host recognition on server, shared PC/PWA cards and Android app-open policy.
- Public mobile metadata requests, using the existing bounded JSON-only hydration parser and pinned HTTPS transport.
- Mobile author-field compatibility; no downloaded JavaScript execution, login cookies or authentication bypass.
- Bounded share-text title fallback for the two new wrappers. Original messages, copying, old wrappers,
  per-platform opening preferences, compact cover geometry, failure states and retry controls remain intact.
- No claim that a note cover grants an embedded-video playback capability.

## Verification

| Check | Result |
|---|---|
| Production-source Rust preview harness | Passed, including domain/lookalike and same-note mobile payload tests |
| Ignored `xhs_cn_live_reported_cards` network acceptance | Both original short links returned ready title, author and inline cover |
| Shared `test-social-link-cards.mjs` | Passed, including old links, new wrappers, comments and async card mounting |
| Android `com.elon.app.sociallinks.*` | 64 tests, 0 failures/errors |
| `test-xhs-short-card-render.mjs` | Real exported covers at 1280/360 px; titles, authors, click targets, image failure and overflow checks passed |
| Production server outbound check | Both selected public note payloads and their matching CDN covers reachable |
| Live PC bundle | Card bundle bytes matched the local build; server/PC completion gate passed |
| Live PWA HTTP and HTTPS | All three embedded media assets matched current source |
| Running Win | Existing semantic MCP `reload_page` completed successfully; not a physical-screen visual assertion |

Live network acceptance is opt-in because external content can expire. Set `ELON_XHS_CARD_EVIDENCE`
to a temporary JSON path to export the public previews, then pass that path to the render test.
Temporary previews/screenshots are not source artifacts and contain no account credentials or request headers.

## Delivery

- Implementation commit: `a6c216ee60d3ebdf46c29446d81312832be4787d`, pushed to `origin/main`.
- Backend and bundled PC frontend: `0.3.1816`; PWA runtime template separately published and verified.
- Android: `1.1.1854` / `1854`, formally published with verified source assets.
- APK SHA-256: `5a6905e5232b9cb430c08593bdb610d7d14ee18cc92ec65af4a044f1273bd4ab`.
- Registered Xiaomi and HONOR were both offline after USB/wireless discovery. Device installation and
  physical Android card acceptance are deferred, not passed. No application data or login state was cleared.
