# Canvas Releases 1675/1677 And Production Acceptance

## Release

- Source: `f291e1b5c72dfaa9d8ff0eb60fe3151aaba7bd98`, adapter 359.
- Published normal `com.elon.app` APK: 1.1.1675, build 1675, 38.38 MB.
- SHA-256: `9add22b70307f725a167b368fcfb17253f05241b3532de394648cf159951e354`.
- Logged release `publish-canvas-native-batch-20260912-043707-599` passed in
  397.5 seconds; Gradle `assembleRelease` passed in 5m52s. Remote hash/size and
  version were verified. The configured Xiaomi target accepted an unattended
  replacement install and reported build 1675. Cookie/data were not cleared.
- Wireless ADB, hardware identity and unlocked display were verified. The real
  social AI ChatGPT surface reported authenticated, ready, adapter 359.
- Release-script background worktree cleanup warned about a missing `Branch`
  property. This is separate from release/install success; the task still uses
  its exact preflight finish contract. No shared cleanup code changed here.

## Actual Results

| Case | Result | Limit |
|---|---|---|
| Native conversation settings -> Canvas | Passed | Semantic click reached the production entry |
| Current conversation Canvas read/empty presentation | Passed | Canonical `canvas_document` succeeded with `canvas_ready`, count 0; native empty row visible |
| Disposable document creation | Not accepted | One actual native send; subsequent Canvas read still returned count 0. No original was edited or shared |
| Native source editor/save/history/restore/first share | Not accepted | No suitable original fixture reached these controls |
| Existing shared Canvas source viewer | Interrupted | Phone foreground changed to `com.elon.quant/.grids.create.NativeGridCreateActivity`; semantic runner rejected it before content acceptance |

Do not interpret the empty fixture result as proof that the account or website
cannot create Canvas. The final script now additionally verifies the fresh send
receipt, exact synthetic user anchor and completed reply parts before consulting
the document list. That strengthened full path has not passed on a device yet.
The original empty-list acceptance does not imply editor or write acceptance.

The phone was independently updated to build 1676 during this window. Git showed
its new source changed grid preparation, not Canvas/ChatGPT adapter code. The
shared-content attempt is a separate, interrupted 1676 attempt, not evidence from
the 1675 installation. No downgrade, duplicate release or foreground contention
was attempted after observing the quant activity.

## Reusable Acceptance Work

- `CanvasUiAcceptance.java` operates package-bound native accessibility actions,
  including collapsed-composer focus, inert body inspection, editor/history
  navigation and cancellation. Optional edits/restores require an explicit
  synthetic-write flag, a bounded fixture marker and an exact body SHA-256.
  Returned evidence is lengths, hashes and booleans, not content.
- `smoke-chatgpt-web-canvas-original-ui.ps1` creates at most one isolated fixture
  per invocation, with explicit opt-in for reversible save/restore. It compares
  native body hashes to save/history/restore results and restores the prior view.
  It does not create or revoke public links; first-share cancellation is planned.
- `smoke-chatgpt-web-canvas-content-ui.ps1` is read-only: original native menu,
  existing account-owned shared selection, source viewer and return navigation.
  Full native body length must match the canonical display payload.
- Main MCP input is `{has_text,text_length,send_enabled}`, not `input.text`.
  Missing input state now fails closed; only a known synthetic draft may be
  cleared. Collapsed presentation must be expanded before using the EditText.
- A native update dialog initially obscured the composer. Earlier runner failures
  did not click send. The first failed report counted a send attempt before the
  click; the final runner increments its counter only after the native click.
- Both scripts check foreground ownership and skip navigation cleanup when
  another app takes over. They do not send Back into that app or reclaim it.
- New smoke contract passed. Existing reply-evidence suite passed 11 cases.
  External Java compiled/dexed and ran on the handset. Unreached write branches
  remain unverified. No production Android source changed in this test batch.

## Follow-up: Landscape And Shared Content On 1677

The two resumed 1676 read-only runs failed before the private content read:
`canvas-content-native-1676-20260912-054717-163` reported
`conversation_actions_missing`; the bounded semantic-scroll retry
`canvas-content-native-1676-scroll-20260912-054927-161` reported
`conversation_menu_scroll_failed`. A filtered native accessibility inspection
showed the 448dp action list extending below the landscape window, with later
rows and footer clipped even at the scroll end. This was a native layout bug,
not a provider capability or authentication failure.

Source `27ac7a9e308f1d505da7a1089ad1201350de3454` gives the existing list a
vertical layout weight so it shrinks before the fixed title/footer. The
acceptance runner reuses its package/window-bound scroll helper to reveal Share.
Eight focused Node checks and source-size/diff guards passed; the layout check
is a source-wiring guard, not an Android rendering simulation.

Normal APK **1.1.1677 (1677)** was published and replacement-installed without
clearing data. SHA-256:
`0a91b751079571a0221f6945944b167a274eaad3b531f3a1490a4558e0d58645`.
Release log `web-chat-action-sheet-1677-release-20260912-060022-728` passed in
485.8 seconds; Gradle completed in 7m7s. Remote hash/size/version and installed
version matched. The existing background cleanup warning was separate from
successful release/install. Adapter remains 359.

**Device result:** `canvas-content-native-1677-20260912-060925-643` passed in
44.9 seconds in production `social_ai`, with a 2400x1080 landscape window
(rotation 270). Native conversation actions -> Share -> existing Canvas list ->
View -> Back to the same link/list and conversation succeeded. The inert native
TextView contained **1392 UTF-16 units**, matching canonical content length;
type was `document`, optional version null. `sent_messages=0`, `writes=0`,
`content_exported=false`, `restored=true`, `awake_restored=true`.

This supersedes the earlier interrupted shared-source-viewer result only.
`android_chatgpt_private_canvas_shared_content_v1:read_and_return` is completed
and device-verified. Source Copy, hot reopen/cancel races, original edit/save/
history/restore, first publication/update and disposable revocation were not
exercised by this read-only run. No private protocol or voice transport changed.

## Remaining Acceptance

Reserve one uninterrupted handset window. Reuse an owned disposable original
Canvas if available; avoid repeated creation prompts or protocol rediscovery.
Run source/editor/history and explicit reversible-write cases, then first-share
publication/update/revoke only on the disposable sample. Keep existing accepted
sharing, models and private transports unchanged. Google and thermal work remain
later phases. The full Goal and incomplete capability markers stay active.
