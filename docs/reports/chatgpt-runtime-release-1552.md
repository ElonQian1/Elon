# ChatGPT runtime release 1552 evidence

Date: 2026-09-08. Delivery and live acceptance are separate results.

## Published artifact

- Source `8ba44319d`, including the resident-composer-host correction and prior
  new-chat confirmation/recovery and mounted-file/export candidates.
- APK `1.1.1552`, code `1552`, global adapter `298`, text-runtime submit `11`,
  runtime bindings `3`.
- APK SHA-256:
  `48a9cb8fa299284e8d62a172758bd8aa51b5220c69c4d6eea045d085eeb01bbb`.
- Release Kotlin/Java compilation and assemble passed in 6m8s; publisher finished
  in 415.1s. Remote publication verification and pinned Xiaomi `install -r`
  passed. Independent package readback confirmed 1.1.1552/1552.
- Packaged text-runtime, private-file-download and private-library-download JS
  matched source after newline normalization. Mounted-file exports are now
  packaged and compiled, not device-accepted file workflows.
- The host correction passed 172 focused and 259 integrated Node runner cases,
  with zero failures, cancellations, skips or todos. New positive-host cases
  failed on the old guard. Previously accepted voice tests were not repeated.

Publisher log: `chatgpt-composer-export-release-20260908-20260908-070947-591`.
Integrated log: `composer-host-integrated-20260908-20260908-070631-304`.
Optional LAN firewall and broad worktree-cleanup warnings did not invalidate
publication or installation. Mandatory task finalization is separate.

## Production text boundary

Both samples used the real social AI native input/send handlers through APK MCP,
not the old test page or direct official-page input. The phone was unlocked,
the voice phase was idle, drafts were empty, and the official account context
was guest. Read-only guards excluded non-fixture messages before any send.

Before replacement, one 1551 send returned the exact synthetic A marker with
one user and one assistant message. Stream observation reached `completed` and
native generating state was false. Receipt still used DOM fallback:
`private_fallback:template_unavailable` and
`runtime_fallback:composer_mode_unsupported`.

After replacement, one 1552 send used the synthetic B marker. The reply matched,
there was exactly one user marker, and the private stream reached `completed`
(revision 8). Receipt again confirmed the DOM send, now with
`private_fallback:template_unavailable` and
`runtime_fallback:submission_not_ready`. The prior host-mode rejection did not
block this attempt. Neither independent private POST nor runtime submission is
accepted by this result.

Device runner log: `runtime-submit-1552-device-20260908-20260908-071738-073`.
The bounded runner ended unsuccessfully after 67.5 seconds because the native
generating flag remained true. Its 60,114 ms sample interval is not a measured
first-token latency: a correct reply was observed while waiting for settlement.

## Settlement inconsistency

Read-only follow-up saw the completed stream and two messages while the native
generating flag stayed true. Later the stream observation became idle and the
message projection briefly contained only the fixture user, then an assistant
entry no longer matching the fixed reply. The sole user remained the test B
prompt. No refresh, reload or second send was issued to explain that transition.
This is a state/projection inconsistency; its root cause is not yet established.

Two attempts to prepare a separate hot sample aborted at pre-send safety guards.
They did not change the input or send another message. Stop request `mcp_4`
failed with the page reporting that no reply was currently generating, while
native UI still reported generating. No failed write was automatically replayed.
The phone was restored to `conversation_home`; no microphone, Cookie, application
data, login or independent VPN state was changed.

## Next acceptance gate

The current public composer uses `rc` both for its exported
`isComposerSubmissionReady` and for explicit `text_action`/prepared-action
acceptance inside `submitComposer`. Its `current_draft` branch uses different
conditions. Removing only our readiness guard is therefore not a demonstrated
solution. Resolve the actual current guest transaction state before widening it.

The stream policy also lets current DOM-active/pending signals retain generating
state despite private completion. Do not globally override those signals with
an old or unrelated completion event: correlate the current owned turn first.
Required next proof is one accepted runtime transaction, one matching reply and
settled native UI without repeat submission. New-chat confirmation, mounted
exports, remaining library variants and other unaccepted scopes stay pending.
Google remains last; the overall Goal is not complete.
