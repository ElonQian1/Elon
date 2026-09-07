# ChatGPT runtime release 1549 evidence

Date: 2026-09-08. Delivery evidence, not new requirements or full acceptance.

## Published and installed

- Source: `a1dcc9ff54c742cede2b6446c31d373b3315f347`.
- Version `1.1.1549`, code `1549`, adapter `294`.
- APK SHA-256:
  `f8e7365cb8c6c1c0b43df6067c477864a547ee5cc7de2d33d04dfda47138d4b2`.
- Publisher completed in 489.5 seconds, including Release build in 7m5s,
  publication and remote version/hash/size validation.
- Whitelisted Xiaomi replacement installation and package readback confirmed
  1549. No application data, Cookie, login state or proxy setting was cleared.
- APK ZIP assets matched the source hashes: file download 10, library download
  4, runtime bindings 2 and text runtime submit 8.
- Log identifier:
  `chatgpt-guest-library-release-20260908-20260908-011735-538`.

The previous 222-case Node integration covers the guest and cache changes.
This release did not rerun Android JUnit; the preceding grouped 1548 report has
46 selected JUnit passes. Neither number proves the live private send route.
Optional LAN firewall setup required administrator rights and optional cleanup
reported a missing `Branch` property; publication and installation succeeded.

## Production probe

The APK MCP operated production `social_ai`, not a developer page. It began at
an empty ready guest homepage with no draft, streaming, dictation or voice.
Exactly one fixed synthetic text was submitted. The native list showed exactly
one user message and one expected assistant reply, then streaming became false.
The correlated send receipt contained:

```text
[private_fallback:template_unavailable] [runtime_fallback:react_owner_unavailable]
```

Guest chat therefore works through DOM fallback. The earlier
`identity_unavailable` gate no longer blocks capture, but direct runtime/private
sending is not accepted. The 295 ms command receipt is not response-completion
or first-token latency. No thermal or battery claim is made.

Before restoration, MCP confirmed both messages still matched only the fixture,
native/official drafts were empty and native voice was idle. The fixture was
reset with the ordinary new-chat command and the app returned to
`conversation_home` with draft length zero. The intermediate wait helper was
called without its mandatory description and did not run; the final home/draft
readback succeeded. No user history was deleted or shared.

## Follow-up source fix

Commit `b30e31e21` adds current-tree membership resolution in runtime submit 9.
It reproduces an actual resolver defect independently of the phone failure:
valid bailout-reused children were rejected, while a stale child's return chain
could authorize obsolete state. Child/sibling membership plus the current root
now chooses the committed context, with bounded traversal and fixed failures.

- Red baseline: 99 tests, 88 pass, 11 fail, log
  `runtime-current-tree-red-20260908-20260908-014732-384`.
- First fix run: 166 pass, zero fail, log
  `runtime-current-tree-green-20260908-20260908-014849-712`.
- Integrated run: 236 pass, zero fail/cancel/skip, log
  `runtime-current-tree-integrated-20260908-20260908-015026-337`.

The browser connection initialized, but homepage navigation timed out. It
supplied no current page-owner evidence. No microphone, user text, credential,
or private browsing storage was read. Submit 9 is not part of APK 1549; its
next grouped device receipt must still establish actual runtime acceptance.
Personal-library downloads and the other documented ChatGPT scopes retain
their separate acceptance gaps. Google stays last and the Goal stays active.
