# Native mixed-media acceptance

## Scope

The existing private uploader and native send owner are reused. No second
uploader, Android HTTP generation client, camera UI or login flow was added.
The authenticated local MCP fixture now offers a pinned text/PNG/PDF bundle,
staged in the production native composer's existing pending-attachment list.
Its metadata and paths are fixed; callers cannot select arbitrary phone files.

Capability checkpoints, not whole-capability completion:

- `android_chatgpt_private_image_attachment_upload_v1`: private batch upload
  and association observed; model image-reading acceptance remains pending.
- `android_chatgpt_private_pdf_attachment_upload_v1`: private batch upload
  and association observed; model PDF-reading acceptance remains pending.
- `android_chatgpt_private_attachment_upload_transport_v1`, scope
  `ordinary_new_mixed_text_png_pdf`: upload/store association passed, final
  native send/reply failed confirmation. This scope is **not completed**.

## Normal 1596

- Source: `02ec5ad6e` (fixture source `c2ba6a6ca`).
- Normal Release `1.1.1596`, code `1596`, 40,013,118 bytes.
- APK SHA-256:
  `3c8b76887155d705636505a42e8ea23aa64fd708fcc0b7a03d30fa4b71d7c461`.
- `publish-native-media-batch-20260909-101004-405`: passed; remote checksum
  matched and the authorized Xiaomi received the normal APK by unattended
  replacement install. Cookies and application data were not cleared.
- 27 targeted Node cases, Release production/unit-test compilation and nine
  JUnit cases passed. The PowerShell acceptance contract passed as well.

The first device attempt stopped at `prepare/foreground_changed`, before any
upload or send. After reopening the native chat, one mixed bundle was submitted
exactly once. The phone was authenticated with an ordinary blank homepage and
no draft, messages or pending attachment. No microphone or private phone file
was used. The prompt did not contain the document markers or image answers.

`native-media-batch-1596-ready-20260909-102340-927` returned:

```text
private_upload=true
local_remove=true
no_early_upload_receipt=true
user_rows=1
attachment_phase=completed
private_send=false
text_read=false pdf_read=false image_read=false
elapsed_ms=120430
restored=true awake_restored=true
```

A read-only in-flight native snapshot recorded
`private_attachment_associated` and
`official_runtime_v1:unknown:context_changed`. Three native rows were present,
but only one was a user row. Thus this is not evidence of a duplicate send.
The `completed` attachment phase also does not prove successful generation.

The upload receipt confirms the existing batch owner associated all three
processed files. It does **not** establish a correct model reply. An unchanged
receipt during staging does not prove that no prewarm network request occurred.
Only local pinned fixtures and the native draft were cleaned; synthetic remote
conversation/library artifacts may remain. No speculative remote deletion ran.

## Acknowledgement boundary

Source review found a reproducible missing transition in runtime submit v17:
an ordinary first send can receive an official dispatch acknowledgement and a
server conversation ID before the router leaves `/`. `sameOwner` then rejects
the ID/URL mismatch. The existing tests navigated before acknowledgement and
did not cover this ordering. Temporary-chat and guest paths already had separate
homepage handling; their evidence must not be generalized to ordinary chats.

Runtime submit v18 permits this narrow **post-acknowledgement** transition only
for the captured new ordinary homepage, with a valid assigned server ID. The
same document, identity, conversation, controller, editor and file-store checks
still apply. Pre-dispatch capture stays strict; another route, account or file
owner is not accepted. Owned-file cleanup still removes only the exact submitted
entries, with no upload, send replay or DOM fallback after dispatch.

52 targeted Node cases passed, including new one/three/nine-file ordering
cases and rejection of unacknowledged or foreign-owner handoffs. The released
v17 source fails the three new positive ordering cases. The isolated baseline
loader also produces an expected old/new module-version assertion mismatch;
that additional failure is a harness-version difference, not another product bug.

The 1596 receipt did not retain per-predicate ownership telemetry. This is a
proven code defect matching the observed error, not proof that it was the only
live failure. A corrected normal APK still needs one real mixed-media reply
before any of these scopes is marked completed. Phone focus moved to another
application during diagnosis; further navigation was left to the next available
device window.

## Remaining Acceptance

Reuse `scripts/smoke-chatgpt-web-media-batch.ps1` for the corrected normal APK.
Do not rerun confirmed voice, dictation, ordinary single-text, library-file or
temporary single-text cases. The mixed fixture checks the production native
handler/preview state and private transfer, not rendered camera or SAF selection.
JPEG/WebP variants, large-file re-encoding, existing/project/temporary mixed
contexts, account upload quotas and temperature are separate, unclaimed scopes.
