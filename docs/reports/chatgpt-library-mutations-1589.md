# Native Library Acceptance: 1589

## Delivery

Normal Release `1.1.1589 (1589)` was built, published and replacement-installed
on the trusted Xiaomi on 2026-09-09. Source:
`80914f60772ff2c9b2acb8c0ad0645b82717807e`; APK SHA-256:
`0e89dd18a4e3ba5a593dc49792351ddcc5c4a50b5dc1ceabc72f176a6ca45ff6`.
Release log: `library-rename-search-release-20260909-052934-830`.
Cookies, application data and login were retained. No diagnostic APK, runtime
asset injection, microphone operation or independent proxy change was used.

## Completed Scopes

- `android_chatgpt_private_library_file_mutations_v1`: ordinary owned text-file
  rename and soft-delete are **completed, enabled and device verified**. Native
  file details, rename field and confirmation invoke the production owner.
  Exact field values and fresh unfiltered directory reads confirm persistence.
- `android_chatgpt_private_attachment_library_reuse_v1`, scope
  `upload_copy_ordinary_text`: **completed and device verified**. A native long
  press selected the explicit upload-copy option for one fixed 78-byte fixture.
  Its private upload returned `private_attachment_associated`; exactly one
  native send received the correct file first line. The original library file
  was kept under a temporary unique name while the independent copy appeared.
  This verifies explicit copy, not a successful hash-reuse hit or all MIME types.
- Catalogue module 6's rename/search reconciliation is **device verified**:
  native rename, fresh directory read, fresh search read retaining the new label,
  then restored name and another directory read. It adds no fetch or polling.

Only the uniquely named disposable copy was moved to Recently deleted. The
private NDJSON owner required `file.deletion.completed`; the native receipt
succeeded and a fresh current-directory page no longer contained the copy.
The original fixture name and original blank chat/draft were restored. No
permanent deletion, user-file deletion or repeated send occurred.

## Evidence And Test Corrections

- `library-rename-search-checks-20260909-052400-132`: 58 focused Node tests passed.
- `library-copy-trash-current-page-1589-20260909-054445-749`: one private upload
  and one reply; the first assertion rejected Markdown-escaped underscores.
  Existing `Normalize-ChatGptProbeReply` correctly recognizes that exact reply.
- `library-copy-trash-finish-1589-20260909-054858-401`: resumed that completed
  send without replay; native rename, soft-delete, directory readback and original
  fixture/chat restoration passed.
- `library-rename-search-live-1589-20260909-055129-357`: native rename/search/
  restore passed, with conversation, draft and message count preserved.

The external semantic UI runner now verifies exact rename input and targets the
clickable ancestor of Android PopupMenu labels. It uses accessibility actions,
not coordinate taps, screenshot polling or a replacement consumer UI. Earlier
menu-label click failures were acceptance-runner failures, not upload failures.
Resumption checkpoints prevented write replay throughout this round.

An acknowledged rename initially looked lost because `nodes?q=ELON` returned an
older name while `nodes` already returned the new one. The current public source
retains the original rename contract. See the
[bounded reconciliation rules](../chatgpt-private-library-mutations.md).

## Still Open

One native next-page read on 1589 ended in `library_read_failed`; this round does
not establish its cause or replace earlier pagination evidence with a claim of
current success. Tests above cover observed current-directory files, not an
exhaustive library inventory. Folder operations, mounted/saved-entity variants,
permanent deletion, restoration, multi-file/project/temporary attachment scopes
and general performance/thermal claims remain separate.
The [remaining work list](../web-ai-private-native-remaining-batch.md) also retains
the failed stop/follow-up flow. The overall Goal is not complete; Google remains last.
