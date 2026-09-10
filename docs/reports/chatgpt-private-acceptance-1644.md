# Native private acceptance on normal 1644

## Installed Baseline

2026-09-11, trusted Xiaomi over wireless ADB, normal 1.1.1644 (1644), adapter 327.
The existing installed source is `0e137a23c73f18308fafa17f53a6ed6c0aff58dc`;
no new consumer APK is needed for these external acceptance scripts.
Preserve the established private transports, identity WebView and original chat.

## Create Image

`native-image-generation-1644-20260911-031244-916` passed in 51 seconds including
setup, generation and restoration. This is not a measured first-image latency.
The corrected `smoke-chatgpt-web-tool-execution.ps1` ran the production native
new-chat/input/send path with an isolated synthetic prompt. Tool selection,
production send receipt, completed assistant output and an image part were
confirmed. It restored the previous conversation and tool state and registered
`reversible/composer_tool_execution/image_generation` as passed.

This closes the corrected full-smoke gap from
[1628's last-row assertion](chatgpt-composer-state-20260910.md#generated-image-reply-evidence).
Reuse `android_chatgpt_native_image_generation_status_v1`; do not implement a
second image-generation transport. The result does not establish saved image
bytes, pixel-level appearance, every model/account scope or a thermal gain.

## Sharing Acceptance Harness

The external `SharedLinkUiAcceptance` uses package-bound accessibility actions,
not screen coordinates. A new synthetic conversation is checked for the exact
prompt and marker reply before any public link can be created. Other user turns,
attachments, truncated/unfinished content, oversized output and wrong contexts
fail closed. A short additional completed assistant row is permitted in the
same synthetic turn; exactly two rows is not an official protocol requirement.

Only the newly created share may be copied/revoked. The account-list receipt
must bind its first row to the exact new share ID and source conversation, and
both native detail and confirmation must display that exact URL. Clipboard
verification pastes into the empty native composer and clears the test draft;
it never sends the pasted URL. A dispatched revoke is not retried. Final
readback compares other link IDs and restores the original conversation/awake
setting. `-ResumeFixture` reuses a local checkpoint instead of sending again.

The preliminary runs stopped before publication: project-origin admission was
too narrow, then the fixture wait/assertion assumed only one assistant row.
Original state was restored and zero links were created in those runs.
Twenty behavioral fixture checks and the script contract now pass. These
checks alone are not rendered Copy/revoke acceptance.

## Acceptance Runner Corrections

The cached native route is deliberately available before its identity page has
finished navigating. `-ResumeFixture` now uses the existing tracked
`open_conversation` completion and exact live route before the sharing test.
The read-only comparison then returned a complete 24-link account index and an
empty fixture-specific list. This was a runner ordering problem, not proof of a
broken sharing protocol; no consumer readiness gate or APK code was changed.

On this Xiaomi, ListView accessibility actions sometimes return false. Selection
now verifies the actual target dialog, with a bounded focus/Move Home/center-key
path that confirms the first row before activating it. It never uses coordinates.
The production composer is collapsed initially; paste acceptance expands its
existing preview and distinguishes accessibility hint text from a real draft.
Cleanup only dismisses a positively identified sharing dialog, not an arbitrary
Back action that could leave the chat. Cleanup failure cannot skip restoration.

Native Copy and later paste readback matched the exact tracked synthetic URL;
the temporary draft was cleared and the production social-AI page remained open.
The preceding run's synthetic link had already been revoked by its cleanup.
That isolated copy check was followed by the complete sequence below.

## Native Shared-Link Copy And Revoke

`native-shared-link-ui-1644-final-20260911-040752-128` passed in 79.3 seconds,
including navigation, external UI runner steps and restoration, not network-only
latency. It resumed the checked synthetic fixture without sending again, created
one official link, returned to the original project conversation, and opened the
production account-wide sharing list. The first row matched the new link and
its different source conversation. Native Copy matched the exact URL when pasted
into the native composer; the test draft was cleared without sending.

The native revoke confirmation was activated once. Its tracked receipt succeeded;
complete account readback confirmed removal and that the other 24 links were
unchanged. Original conversation, empty draft and awake setting were restored.
No Cookie, application data, credential or conversation content was exported.

Scoped case `android_chatgpt_private_conversation_shared_links_v1:account_row_copy_cross_conversation_revoke`
is completed/device-verified on normal 1644. Reuse the existing enabled transport;
no consumer code, transport version or APK was changed in this acceptance batch.
Workspace/Canvas/post/bulk variants, large-account page controls and server-side
pagination remain unverified. The external Java class compiled/dexed successfully;
20 fixture behavior cases, the runner contract and source-size guard passed.
