# Independent Image Send: 1739

Capability: `android_chatgpt_fresh_tool_text_dispatch_v1`.
Scope: authenticated existing personal conversation, selected `picture_v2`,
no attachments. This completes the Image half of the existing scoped capability,
not new/project/temporary/tool-plus-file combinations.

## Actual Production Evidence

Installed APK `1.1.1739 / 1739`, adapter 405, source
`80d4674d12913b6b4b2aff29685bf343aa4fa94c`, SHA-256
`067503dbea86c41825908a62c803c0c3b7b141c0ebf5f45865a94192a8256ae9`.
Reused the resolved, owned fresh-text fixture and tool ledger. The earlier Search
pass was not repeated; no new seed conversation or seed send was created.

`fresh-image-native-origin-1739-20260915-012133-018` ran the existing
`smoke-chatgpt-fresh-tool-dispatch.ps1` via semantic controls in the production
native chat. The bounded one-command trial was armed only for Image admission.
Its `elon.fresh_tool_ui.v1` result passed:

- One acknowledged native Send action, one send receipt, zero seed sends.
- Independent private HTTP accepted; 27 stream events and reconciled history.
- Completed Web image output matched the native image part and source-message
  ownership for that exact synthetic user turn; not an old or partial reply.
- Image selection cleared with confirmed state, original ChatGPT conversation
  restored, screen-awake setting restored, no uncertain write outstanding.
- Observation took 33,090 ms including generation and polling. This is not TTFT,
  a power/thermal benchmark, image pixel inspection, or a provider-wide claim.

No prompt, conversation ID, credentials, request headers or private content is
included in this report. The existing completed fixture ledger prevents repeats.

## Harness Failure And Recovery

The first preflight (`fresh-image-native-1739-20260915-011922-316`) required the
home surface but found the user's idle project chat. It stopped before any send.
The wrapper was then allowed to retain/restore that existing route.

Although the inner Image acceptance passed, the later outer wrapper exited 1:
its final `open_project_chat` action retained the social-chat surface when the
same remembered project/conversation IDs were selected. This is not a passing
whole-wrapper run. No Image request was repeated.

A separate safe return through `show_conversation_home`, then
`open_project_chat`, restored the remembered original project chat. Readback
confirmed `restored=true`, `surface=project_chat`, empty draft and zero sends.
The unrelated MCP navigation implementation was not changed in this batch.

## Default Source Change And Checks

Context v15 and transaction v26 admit this accepted existing-personal Image scope
without a trial, parallel to Search. Explicit tool-off and global switches still
win; new/project/temporary/attachment combinations remain gated. Request bodies,
stream handling and uncertain-write protection are reused unchanged. Adapter406
activates the new assets in the next grouped release.

Private-input v4 mirrors the same scope and removes the inherited cooldown only
when recapturing a previously successful personal tool binding. It never lets a
changed tool edit using the old binding; a failed fresh capture still backs off.
Attachment ownership changes retain their earlier behavior.

`fresh-image-default-final-20260915-013615-081`: 315 tests passed, zero failures or
skips, 3.9 seconds. Tests cover both hint permissions, default opt-out, unknown
permissions, scope drift, project routes, new/temporary/file exclusions, one POST,
correct Picture metadata, fresh Search/Image input binding and failed-capture
backoff. Earlier failing runs are not counted as passing evidence.

These admission/input edits are not yet compiled or installed. Group publication
with the next coherent Android batch; do not repeat the accepted image generation
merely to confirm packaging. The phone remains on 1739 until that publication.
