# Generated Image Files: Normal 1684 Acceptance

Date: 2026-09-12. Production Android UI acceptance, not a test chat page.

## Accepted Scope

| Capability ID | Status | Verified boundary |
|---|---|---|
| `android_chatgpt_generated_image_file_index_v1` | `completed`, `device_verified`, default-enabled | Owned current generated image appears in native conversation Files and saves its original |
| `android_chatgpt_generated_message_original_v1` | `completed`, `device_verified`, default-enabled | Owned inline final image opens from a native message bubble and saves through Download Original |

Reuse these scopes without repeat research unless a current regression appears.
Protocol and source checks remain in the [history](chatgpt-generated-image-history-20260912.md)
and [bubble-original](chatgpt-generated-image-original-20260912.md) reports.

## Build and Installation

- Normal published APK: `v1.1.1684 (1684)`, adapter `364`.
- Application source: `bd9998bb2307fc3ab3543204116c01bad0c62134`.
- APK SHA-256: `f68f0bdcd79572c5c02491dd2917011baafa382b27e99461bfbcec0a7b366ee9`.
- Standard publisher completed build, server verification and replacement install
  on the already trusted Xiaomi 14 Pro. No Debug APK or browser-only acceptance.
- Release unit tests passed: 29 tests, zero failures/errors across
  `WebChatImageOriginalTest`, `WebChatConversationFilesPresentationTest`,
  `ChatGptWebConversationFilesTest` and `ChatGptWebFileDownloadSessionTest`.

## Actual Native UI Results

The existing owned generated-circle fixture was checked before use. No new
message or generation was sent. Semantic UiAutomator controls exercised the
same consumer UI; MCP provided state and command receipts, not a bypass download.

| Path | Result |
|---|---|
| Header / current settings / Files / image row / Download | One generated image row; exactly one `download_conversation_file` receipt, `succeeded` / `download_saved` |
| Message image / native viewer / Download Original | Actual image and original button visible; exactly one `download_conversation_file` receipt, `succeeded` / `download_saved` |

Each path saved one new **671,371-byte PNG, 1254 x 1254**. Phone-side checks
validated PNG signature, terminal IEND, bitmap bounds and sampled decode.
Files remained on the handset; no image bytes, signed URLs or credentials were
exported. This proves matching dimensions/size and successful decode, not a
byte-for-byte hash comparison between the two saved files.

Idle expiry removed the opaque file handle. The native Refresh control restored
it and the file-list download passed. Automatic expired-handle refresh was not
verified. Foreground guards correctly rejected intervening switches to another
app; returning to the production app allowed the bounded cases to continue.

After acceptance, the original conversation, message count, draft length and
awake setting were restored. The official draft was empty. No audio recording,
Cookie/app-data clearing or independent proxy changes were performed.

## Evidence and Remaining Boundaries

`scripts/android/LibraryUiAcceptance.java` now provides
`conversation_image_download_verified`, `message_image_preview`,
`message_image_download_verified` and `message_image_close_preview`. It is an
external test runner, not code shipped inside the consumer APK. Both new paths
were compiled and executed against normal 1684.

This acceptance does not cover ordinary uploaded-image originals, old DALL-E
owners, compact/shared/project variants, separate paid/free watermark cases,
large-transfer cancellation or independent generated transcript rendering.
Superseded file-read ownership was included in the APK, but its race was not
separately reproduced. Canvas/Writing Blocks and the remaining private-native
work list remain open; this is not full Goal completion.
