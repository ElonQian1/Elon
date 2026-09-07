# ChatGPT private common-document upload

Capability: `android_chatgpt_private_common_document_attachment_upload_v1`.
Implementation: **implemented for 87 explicit document MIME types**.
Verification: **offline contract tests and Kotlin policy/reader tests passed;
full Android compilation and device acceptance pending**. Delivery: **source-only
for the grouped APK**. This extends the existing uploader, not a second transport.

## Current source evidence

The following public official assets were re-inspected on 2026-09-07. Their
locally retained bytes were hash-checked; they are protocol source evidence,
not a live upload or proof of this account's permissions.

- `https://chatgpt.com/cdn/assets/conversation-small-hiw4wce20lu6te81.js`, SHA-256
  `296ec15ad991764de750c55f3c85b1643c8f385236b9402168fa4348696e37d1`:
  retrieval attachment preset `zw` lists 75 explicit MIME types, including code,
  structured text and Apple documents. `Fqt` categorizes documents; its `Bqt`
  HWP/HWPX set contains seven MIME aliases. `qqt`, imported as `DIe`, resolves
  an AceUpload use case to MyFiles for HWP/HWPX filename suffixes. This is not
  a reason to replace every request's original use case; see the scope below.
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

The native File action now admits 87 explicit document MIME types. The initial
18-type text/PDF, Office, ODT/RTF and structured-text set is extended with the
evidenced source-code, script, CSS/template, SQL/YAML/TOML, email/calendar/contact,
Pages/Keynote and HWP/HWPX types. MIME aliases are not independent file formats.
The exact shared cases live in
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

For HWP/HWPX, the existing reservation module already matches the official
distinction: retain the original intended AceUpload scope, claim MyFiles, and
request retrieval for that override. The legacy create/process path retains
its original use case and ordinary retrieval rule. This change admits the bytes
to those existing paths; it does not add another transform or upload protocol.
Ordinary/temporary reservation integration tests exercise all seven MIME aliases.

Current activation versions: attachment protocol 11, send owner 16 and page
adapter 289. Owner reinjection retires older captured dependencies, while a
second injection retains the current owner. The grouped APK must include these
assets and the native MIME policy together. Source commits: `8f932887d` (policy
and integration) and `59f312926` (reinjection and activation).

## Verification and remaining acceptance

On 2026-09-07, all private-attachment Node suites passed **212 test-runner cases**
with zero failures, skips or cancellations. After the activation version bump,
the composer/document/reservation integration subset passed **60 cases**.
Both runs include production asset wiring; reservation integration also parses
the complete production asset bundle. Fixture-driven cases cover the actual byte
source, composer, transport and parser across ordinary/temporary/new/existing
chats and project permissions, retrieval and origins. Binary boundary checks
compare every byte across 64 KiB chunks. Incomplete processing or an unknown
upload strategy retains unconfirmed state without replaying writes. Expanding
the fixture first reproduced the old MIME policy and byte-source rejection.
HTTP/runtime/file bytes are synthetic, not valid Office/HWP/Pages documents or
real account API responses.

The current Kotlin policy, byte reader and their current tests were compiled
together and passed **11 JUnit tests**. The shared fixture covers MIME parity,
size bounds and rejected types; reader tests cover cancellation and byte leases.
This used fresh pure-JVM classes, not cached app classes or an Android build.
Logs are `document-types-all-attachments-20260907-145121-597`,
`document-types-reinjection-20260907-145256-827` and
`document-policy-kotlin-20260907-144845-787` under the repository's Git command
log directory. No Gradle, APK publication, phone or microphone operation occurred.

For the grouped production acceptance, use small valid Word/Excel/PPT fixtures
from the native File action. Retain the private-association receipt and verify
one actual file-reading reply, cancellation and exact project/library placement.
Do not count upload completion alone as proof that the service read the file.
Single-file 8 MiB limits remain. Explicit MIME admission does not establish
account/model permission or backend reading support. Unknown/ambiguous MIME,
cloud-native references, archives, audio and video remain outside this extension;
there is no filename-based guess for an unknown Android MIME. Multipart,
reservation and library reuse are separate existing candidates, not missing
transports to reimplement here. Existing compatibility selection precedes private
writes; uncertain writes are never automatically replayed. See
[the remaining batch](web-ai-private-native-remaining-batch.md).
