# Library And Local Picker Acceptance 1637

## Artifact And Scope

Normal Release **1.1.1637 / code 1637**, source
`ea470626596faf40147439f98a49126a9a9f02c5`, adapter 321. APK SHA-256:
`364de9aba1d0c4e051a7163cede011d121fdca55934c9d5517fb441c03e59264`.
The grouped build/publish passed in
`webchat-grouped-321-release-20260910-175241-150`; server bytes and wireless
replacement installation were verified. No Debug/test APK, data reset, Cookie
clear, login change or proxy-core modification was used.

On 2026-09-10 the Xiaomi production friend-chat surface accepted these scopes:

| Capability | Code | Verification | Completion |
|---|---|---|---|
| `android_chatgpt_private_library_selection_refresh_v1` | implemented | device_verified, ordinary visible TXT selected beyond TTL | completed for this scope |
| `android_chatgpt_private_library_local_upload_append_v1` | implemented | device_verified, Library TXT then local TXT, one native send | completed for this scope |

Both use the normal default path. Do not repeat these scopes without regression
evidence. Existing 1635 consecutive-Library acceptance remains reusable.
The closed structural receipt is retained under Git metadata at
`ai-research-artifacts/library-local-append-1637/acceptance.json`; it contains no
conversation paths, account identity, headers or file contents.

## Actual Consumer Flow

1. Opened the production `social_ai` ChatGPT chat with adapter 321, ready composer,
   idle voice and empty draft/attachments. Saved the exact original conversation
   path locally without printing it, then opened an ordinary blank conversation.
2. Opened the native sidebar and file library. The external semantic UI runner
   searched only `elon-chatgpt-attachment-fixture-v1.txt`, selected its exact
   visible handle and opened the actual file-detail sheet.
3. Waited **67 additional seconds** before pressing its actual Attach button.
   At 16, 31, 47 and 67 seconds the exact foreground, conversation URL and library
   query remained unchanged. The native command succeeded and one ready official
   attachment appeared. No new search, refresh click or page navigation was
   inserted during the wait. This is live aged-selection association evidence;
   the exact network count is not inferred from the receipt.
4. The library automatically returned to the composer after association. Opened
   the production `web-chat-attachment:chatgpt_web` control and
   `attachment-action-files`. On this phone Android dispatched to Xiaomi's
   `com.android.fileexplorer/.picker.PickMainNavigatorActivity`.
5. Used its actual Browse tab, Download directory, fixed local TXT row and
   confirmation button. The fixture originated in the committed
   `scripts/fixtures/chatgpt-local-append-fixture-v1.txt`; only those synthetic
   bytes were staged in Downloads. The consumer file-picker callback, not the
   MCP fixture-staging shortcut, created the native pending attachment.
6. Both native attachment presentations were visible. MCP confirmed the same
   original Library attachment ID and exactly one local composer file. The
   request asked for both files' first two lines without supplying their values.
   Pressed the actual `web-chat-send` button exactly once.
7. A fresh `request_attachment_upload` receipt reported
   `private_attachment_associated`, `ok=true`. The text receipt reported
   `send_prompt`, `official_runtime_v1:accepted`. The reply contained the Library
   fixture marker and both the new local marker and its independent second-line
   value. This proves both files were available to the same request, not merely
   that attachment cards rendered.
8. Official conversation state contained exactly **one user and one assistant
   message**. Both pending collections and the draft were empty. Restored the
   original conversation, confirmed it again, and removed only the exact local
   Downloads fixture after matching its SHA-256. The synthetic server test
   conversation remains; no unrelated server record was deleted.

## Harness Corrections And Boundaries

The external picker runner reuses `invoke-android-semantic-acceptance.ps1`,
compiles outside the APK and uses accessibility actions, not screen coordinates.
It restricts fixture names, package ownership and send consent. Inspection emits
resource IDs and allowlisted navigation labels, not arbitrary file names or text.
Java compilation, PowerShell parsing, send-consent rejection and the above actual
picker/send path passed. It creates no consumer test screen.

Early harness attempts do not represent product failures: the Library entry
requires the native sidebar open; successful association closes the browser
asynchronously; Xiaomi uses its own picker; its Internal Storage heading is not
a button, and directory scrolling belongs to the file list, not the outer sheet.
These boundaries were inspected before retrying the relevant UI step. No failed
attempt sent a message or automatically replayed a write.

The controller's upload-pending count is not the unsent local composer count.
This acceptance used `chatgpt_web_acceptance_attachment.composer_pending_count`
and actual attachment previews for pre-send ownership, then the fresh upload
receipt for transport proof. The original synthetic-fixture staging API was not
used to bypass the picker.

The following are **not** newly accepted: mounted/cloud files, reverse-order
multi-local batches, image/PDF variants, project/temporary Library references,
quota exhaustion, failed/changed refresh responses, standalone remove-only TTL
flow, or standard Android/Google DocumentsUI picker actions. These remain their
existing offline or separate verification scopes. Thermal benefit was not measured.
