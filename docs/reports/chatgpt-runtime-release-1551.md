# ChatGPT runtime release 1551 evidence

Date: 2026-09-08. This report records delivery separately from acceptance.

## Published artifact

- Source: `66e0fe447`, including deep-composer resolution `e6f6723e4`, background
  discovery backoff `a5af5dfbf`, action bindings `c76828797`, and registered
  new-conversation navigation `66e0fe447`.
- APK `1.1.1551`, version code `1551`, global adapter `294`, text-runtime `10`,
  runtime bindings `3`, new-conversation runtime `1`.
- APK SHA-256:
  `2ae2a226345e1ce8b221f76d1530ed14993ac306822fa919078352aa1fe508ca`.
- Release compilation/assemble passed in 7m11s; publisher completed in 472.9s.
  Remote publication verification and Xiaomi replacement installation passed.
  Independent package readback returned version name/code 1.1.1551/1551.
- ZIP entries for the new-conversation, text-runtime and runtime-binding modules
  exactly matched the committed source after newline normalization.
- The integrated navigation/runtime/consumer suites passed 277 Node runner
  cases. No full Android regression or previously accepted audio test was rerun.
  The preceding backoff batch separately passed 28 selected Android tests.

Publisher log stem:
`chatgpt-navigation-retry-release-20260908-20260908-032523-041`.
It warned about optional LAN firewall setup and optional broad worktree cleanup;
those are not APK publication or installation failures. Mandatory task cleanup
is performed separately, without changing unknown main-worktree files.

## Device boundary

Before installation, production MCP reported `conversation_home` with zero
native draft and no attached web snapshot. Absent web fields were not treated
as proof of a voice state or an authenticated account.

After installation, MCP bootstrap and then a plain bounded ADB hardware query
timed out. Reconnecting the already-connected transport alone did not repair
the shell. Disconnecting/reconnecting only the pinned Xiaomi wireless transport
once restored the exact hardware identity and package query. No global ADB,
app, phone, VPN or account reset occurred.

The recovered display check reported asleep and keyguard showing. The user was
asked to unlock. No new-chat action, test message, attachment, microphone or
private capability acceptance was attempted on this installed artifact.
Do not promote text-runtime or registered new-chat capability based only on the
installation. The 1550 fallback-send result remains the latest actual send result.

## Recovery conflict found after publication

Code review traced the production new-chat owner to a separate three-second
native recovery timer. While navigation was active/loading and the editor was
absent, it could call `stopLoading` plus `reload` or `loadUrl` before the page
command's five-second settlement completed. That conflicts with page-owned
guest confirmation and the no-automatic-replay boundary. It is a source-level
finding, not a device observation of a lost guest conversation.

The follow-up removes WebView mutation from this coordinator entirely. It can
only request one fresh snapshot when the existing readiness signals still need
it. The active page command keeps navigation/confirmation ownership; ordinary
failure restores the existing retained snapshot. There is no new repeating
timer, no automatic retry and no change to independent network recovery.

Red verification compiled and ran three policy tests; both forced-navigation
branches failed the new expectation. Log:
`new-chat-recovery-red-20260908-20260908-033736-651`.
Follow-up Release production/test compilation and 19 targeted JUnit tests passed:
recovery policy 4, navigation actions 5, navigation coordinator 7 and boundary
policy 3, with zero failures/errors/skips. Green log:
`new-chat-recovery-green-20260908-20260908-034429-750` (291.8 seconds).
This repair is not in 1551; it is a source candidate for the next grouped build.
Native guest-confirmation UX, installed new-chat/text route, idle-request rate,
thermal impact and the remaining private scopes are still pending. Google stays
last and the full Goal remains active.
