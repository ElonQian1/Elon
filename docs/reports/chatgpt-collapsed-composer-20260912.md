# Native collapsed composer evidence

Date: 2026-09-12. Scope: production ChatGPT native composer. This is a UI
correction on the existing official-runtime sender, not a new private HTTP
transport or acceptance of Writing Blocks/Canvas.

## Baseline

Normal APK 1.1.1682 / 1682, adapter 361, was tested over wireless ADB with the
trusted-device guard. No microphone, login, Cookie, VPN or application-data
operation was performed.

- A footer-scoped Accessibility click opened the native editor and retained
  focus after two seconds. The previous global text selector was unreliable;
  do not modify production focus behavior to compensate for that test issue.
- `smoke-chatgpt-web-collapsed-composer.ps1 -CreateFixture` created an isolated
  empty conversation, typed a synthetic prompt into the actual native editor
  and pressed Back to close the keyboard.
- The draft remained visible, but Send was absent and realtime voice was
  visible: `collapsed_draft_send_missing`, `sent=0`.
- Synthetic draft cleanup, original conversation and awake-lease restoration
  all passed. No prompt was sent by the failing baseline.
- Evidence: logged-command run `collapsed-composer-baseline`, 21.6 seconds.

## Correction

`WebChatProductionComposerVisualModeResolver` accepts an explicit opt-in for
sending collapsed text drafts. `MainSendButtonVisualActions` enables it only
when the current composer belongs to web chat. Work-mode defaults are unchanged.
Streaming Stop and active dictation completion controls retain priority;
explicit voice-input mode is not converted into Send.

The semantic acceptance helper now scopes collapsed previews to `inputLayout`.
It checks editability, distinguishes hint text from user text, changes only an
empty or owned synthetic draft and invokes the actual native Send action.
MCP is used for receipts/context verification and navigation, not to substitute
for native text entry or Send. Context reads use the observed message index.

## Verification

- Baseline failure reproduced and restored as above.
- PowerShell syntax/native-action guard and existing Canvas guard passed.
- Release Kotlin/Java and unit-test compilation passed; all seven
  `WebChatProductionComposerVisualModeTest` cases passed with no skipped tests
  (`collapsed-composer-unit`, 320.7 seconds).
- Source-size and document-modularity guards passed.
- Post-fix normal APK acceptance is pending.

Acceptance must prove collapsed Send visible, voice hidden, exactly one native
send, a fresh `official_runtime_v1` receipt, the matching completed synthetic
reply, and restoration. A published/installed APK alone is not a pass.
