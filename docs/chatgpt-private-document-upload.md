# ChatGPT private common-document upload

Capability: `android_chatgpt_private_common_document_attachment_upload_v1`.
Implementation: **implemented**. Verification: **offline contract tests passed;
Android compilation and device acceptance pending**. Delivery: **source-only for
the grouped APK**. This extends the existing uploader, not a second transport.

## Current source evidence

The following public official assets were re-inspected on 2026-09-07. Their
locally retained bytes were hash-checked; they are protocol source evidence,
not a live upload or proof of this account's permissions.

- `https://chatgpt.com/cdn/assets/conversation-small-hiw4wce20lu6te81.js`, SHA-256
  `296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`:
  attachment preset `zw` lists document MIME types; `Fqt` categorizes PDF,
  spreadsheets, presentations and office documents. `qqt`, imported as `DIe`,
  preserves the selected upload use case except for the separately scoped
  HWP/HWPX override. That override is not enabled by this change.
- `https://chatgpt.com/cdn/assets/8b34dbc2-kjj15hg4y6iyx13p.js`, SHA-256
  `9990fb9a8682917d0d790acf7b6aa78355e8520e4ffd2c5e0a183212d612d4b5`:
  `VGt` uses the common create/upload/process flow; `SGt` retains the declared
  MIME, name, size, scope and persistence fields. Only PDF receives the model
  header. `wGt` applies project ingest retrieval to spreadsheet suffixes or
  gated images. `TGt` preserves the existing project/origination metadata.
- `https://chatgpt.com/cdn/assets/4813494d-hrplraurzfyvxb10.js`, SHA-256
  `89c95d937bac1191e91d5ceb4872eb0c328d39a98ce05399093a663f18921aa0`:
  `EEt`, imported as `Cie`, recognizes `xls`, `xlsx`, `csv` case-insensitively.
  The file MIME sets also include the spreadsheet/CSV/TSV/XML categories. TSV
  is not silently treated as CSV for project retrieval indexing.

## Native production path

The native File action now admits 18 explicit document MIME types, including
existing text/PDF plus Word, PowerPoint, Excel, ODT, RTF, CSV/TSV, Markdown,
JSON, XML and HTML. The exact shared cases live in
`android/app/src/test/resources/chatgpt_private_attachment_documents.json`.

`ChatGptWebNativeAttachmentPolicy` admits the file before the existing native
byte lease. The page's versioned protocol supplies one frozen document list to
both the byte reader and project scope helper; a source contract test checks
parity with the native list. The sequential bridge preserves binary bytes and
the declared MIME, including office container formats. No decoding, image
conversion, system service, extra picker or replacement HTTP client is added.

Ordinary and temporary chats reuse their current ownership/persistence rules.
New/existing writable and read-only project chats reuse fresh permission and
selected-branch checks. Excel/CSV names retain the evidenced ingest retrieval
flag; other documents do not wait for the image-only runtime gate. Read-only
chats retain conversation origin but omit project-write metadata. The exact
ready-file object is associated with the official composer store before success.

Versions: attachment protocol 9, native byte source 4, project scope 6 and send
owner 8. Owner reinjection retires older captured dependencies. The grouped APK
must include these assets and the native MIME policy together.

## Verification and remaining acceptance

Eight focused Node suites passed **115 test-runner cases**, with zero failures,
skips or cancellations, including complete production asset-bundle parsing.
The new tests execute 72 ordinary/temporary/new/existing document combinations
through the actual byte-source, composer, transport and response parser modules;
144 project combinations cover permissions, retrieval and origins. Binary
boundary checks compare every byte across 64 KiB chunks. Failure cases retain
unconfirmed state after incomplete processing or an unknown upload strategy and
never replay writes. Source-policy, byte and project regressions were observed
before their corresponding fixes. HTTP/runtime/file bytes in these checks are
synthetic, not real valid Office documents or account API responses.

The Kotlin policy test consumes the same fixture and checks size bounds and
uncovered types. It was added but **not compiled or executed in this batch**;
the earlier 1541 tests do not validate this change. No Gradle, APK publication,
phone operation or microphone use was performed.

For the grouped production acceptance, use small valid Word/Excel/PPT fixtures
from the native File action. Retain the private-association receipt and verify
one actual file-reading reply, cancellation and exact project/library placement.
Do not count upload completion alone as proof that the service read the file.
Single-file 8 MiB limits remain. Unknown/ambiguous MIME, HWP/HWPX, cloud-native
pointer formats, archives, multipart/reservation/direct-library variants and
other document categories remain outside this extension. Existing compatibility
selection precedes private writes; uncertain writes are never automatically
replayed. See [the remaining batch](web-ai-private-native-remaining-batch.md).
