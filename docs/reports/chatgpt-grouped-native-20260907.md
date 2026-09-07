# ChatGPT grouped native release 1548

Date: 2026-09-07. Delivery and verification evidence, not new requirements.

## Published artifact

- Source commit: `cdfe8e6b67d9a15381d638241b0291486323f491`.
- Release: `1.1.1548`, version code `1548`, ChatGPT adapter `294`.
- APK SHA-256:
  `6579e0d997d5cf87d5b5bc1a5769373eede8ecd1775b606964ffae677fc6e39f`.
- The repository publisher completed Release compilation, packaging, remote
  publication and version/hash checks. Total logged duration: 511.4 seconds.
  Independent remote metadata and local APK hash matched afterward.
- During publication the previously absent Xiaomi returned over wireless ADB.
  The publisher replacement-installed the release, and a separate pinned-device
  package query confirmed `1548` / `1.1.1548`. No data/Cookie reset was performed.
- Four APK assets were read directly from the ZIP and SHA-256 compared with
  source: private model catalog, model state, model contract and runtime bindings.
  All four matched. Installed MCP subsequently reported adapter `294` current.
- Adapter 294 was already the unpublished increment after 1547/293. This grouped
  artifact ships that increment and the subsequent catalog additions together;
  no unrelated version-owner edits or per-module APK releases were needed.

The release includes the [runtime owner/status fixes](chatgpt-runtime-owner-status-294.md)
and [additional model catalog](../chatgpt-private-model-catalog.md). Previously
verified audio, subtitles, dictation and read-aloud were not changed or retested.
The independent proxy was not modified.

## Android verification

Release main and test Kotlin compiled. Seven selected JUnit classes ran **46
tests, zero failures/errors/skips**: model control policy (6), range policy (2),
consumer composer state (5), production composer commands (4), model catalog
protocol (1), composer option semantics (2) and web protocol (26).

The first run had one failure: the new catalog test passed a bare event instead
of the actual `schema: yilong.ai.ui.v1` / `event` bridge envelope. Production
correctly rejected it. The test fixture was corrected, not the production parser;
the identical seven-class run then passed in 113 seconds.

Evidence identifiers:

- Initial log: `chatgpt-grouped-native-tests-20260907-20260907-230751-073`.
- Final log: `chatgpt-grouped-native-tests-fixed-20260907-20260907-231718-370`.
- JUnit archive: `junit-models-20260907-231718.zip`, SHA-256
  `a17e402b42fd2b486ea73d06a474c8d764112bea3a3a070c6d3931407228c189`.
- Publication log: `chatgpt-grouped-release-20260907-20260907-232249-300`.

Logs and generated evidence remain under the repository's Git metadata artifact
directories. This is selected Android coverage, not all tests or all features.

## Production device result

The existing APK MCP opened the real `social_ai` ChatGPT consumer surface, not
the retired developer page. The device was unlocked. The initial state was an
empty ready homepage, zero native/official draft characters, no streaming,
dictation or realtime voice. It reported `authenticated=false`, no forced-login
requirement, and later three visible sign-in controls were found in its manifest.

A single fixed synthetic prompt was sent through production `set_input_text`
and `send_input`. One user message and the exact expected assistant response
appeared, streaming stopped, and the native list contained exactly two messages.
Total observed completion time was **4526 ms**. The test reset only its own
synthetic homepage session and returned to `conversation_home`; restoration was
confirmed. No existing conversation was shared, deleted or overwritten.

The successful send receipt explicitly reported:

```text
[private_fallback:template_unavailable] [runtime_fallback:identity_unavailable]
```

Therefore this is a **guest production-chat/fallback pass**, not direct private
HTTP or official-runtime send acceptance. The exact runtime failure gate is now
retained as intended. No extra progress bubble occurred in this one probe, but
that does not establish all authenticated streaming/voice rendering cases.

## Remaining acceptance

- Confirm a signed-in ChatGPT session before accepting account-specific private
  model catalogs and guarded runtime submission. Do not weaken identity guards,
  force login for guest chat, or claim that no identity means no provider feature.
- With that session, require an actual `official_runtime_v1:accepted` send
  receipt; verify model catalog selection/restoration only when its official
  account catalog has a selectable entry. These cases were not attempted here.
- Remaining attachment scopes, gallery, tools, temporary-mode contexts and
  regeneration retain their separate implementation/acceptance gaps. Reuse the
  already completed 1547 share/revoke/delete cases; Google remains last.
- No thermal, battery or broad latency improvement was measured by this probe.

Publisher warnings: optional worktree cleanup still reports a missing `Branch`
property; LAN firewall setup requires administrator rights. Neither invalidated
remote publication or the separately confirmed device update. Mandatory task
cleanup is recorded by the task finisher, independently of those warnings.
