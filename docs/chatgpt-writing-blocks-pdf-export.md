---
capability_id: android_chatgpt_writing_block_pdf_export_v1
implementation_status: implemented
verification_status: production_verified
delivery_status: published
completion_status: completed
acceptance_scope: native_pdf_copy_export
---

# Writing Block PDF Copy

Grouped build **1723** is published and installed. Production native PDF copy
export, saved bytes, parser/visual inspection and source/view restoration passed
after an interrupted run and correction of a test-only label matcher. See the
[grouped delivery report](reports/chatgpt-writing-grouped-acceptance-20260914.md).
The exact Unicode extraction limitation below remains; external phone-reader
opening was not tested. Reuse this completed export scope without re-testing
unless a current regression appears. Earlier source-only notes are historical.

This extends the existing production native Writing Block editor with an
explicit PDF export option. It is a **local document copy**, not the provider's
Canvas PDF endpoint, not a screenshot, and not a cloud-save receipt.

## Implementation

- `WebChatTextBlockPdfContent` parses the same CommonMark dialect as Word export:
  headings, paragraphs, emphasis, strike-through, lists, quotes, code and tables.
  Code spacing is retained; unsupported input is rejected, not silently dropped.
- `WebChatTextBlockPdf` creates text/vector pages through Android `PdfDocument`
  and `StaticLayout`. Line-boundary pagination continues long paragraphs, code
  and table cells without truncation. Wide tables use landscape A4; ordinary
  documents use portrait A4. Native text shaping handles Unicode and direction.
- Hyperlinks and images retain their labels and destinations as inert text.
  No remote image is fetched, no HTML is executed and no private data is sent.
- The existing native export menu, background export worker, filename policy,
  download publication, failed-write cleanup and explicit Open/Share actions are
  reused. No Activity, page adapter or private-write owner is changed.
- Incomplete blocks, malformed Unicode and mismatched format/MIME are rejected.
  Existing 120,000-character input limits remain; traversal is capped at 30,000
  nodes and 64 levels, tables at 32 columns, output at 300 pages and 16 MiB.
  Failure leaves the editor's copy intact and never publishes a partial file.
- Markdown, TXT, Word and code-source exports remain available. Code blocks keep
  their source-file export choices; this PDF extension is for Writing Blocks.

Android API contracts:
[PdfDocument](https://developer.android.com/reference/android/graphics/pdf/PdfDocument),
[StaticLayout.Builder](https://developer.android.com/reference/android/text/StaticLayout.Builder).

## Acceptance

`writing-pdf-structural-regression-20260914-123635-392` passed Release Kotlin/Java
compilation and 31 targeted tests, with zero failures/errors/skips, in 102.9 s.
This includes existing Word/text/native-menu contracts plus PDF content, styles,
Unicode/code preservation, native text-layout line continuity and rejected input.

The initial run (`writing-pdf-native-tests-20260914-122139-000`, 355.9 s) compiled
successfully but failed two actual-PDF tests before pagination: Robolectric
4.16.1 has no PDF implementation in its shadows JAR; `PdfDocument` receives a
zero native handle and reports `document is closed!`. Those cases are not
marked passed or hidden behind skips. They moved to the Android-native runner
below, which additionally covers tall table-cell continuation.

`scripts/test-chatgpt-text-block-pdf-native.ps1` packages only the compiled
production PDF modules and resolved Kotlin/CommonMark dependencies, then runs
four synthetic cases via Android `app_process`. It opens each result with
`PdfRenderer`, checks nonblank body pixels and A4 dimensions, and retains a
synthetic PDF/PNG for inspection. It never installs an APK, changes foreground
UI, reads application data or accesses an account. A unique shell-owned scratch
directory is cleaned only after successful creation.

`writing-pdf-harness-compile-20260914-123537-656` passed Java/D8 compilation in
12.4 s, payload SHA-256
`7c8a528ccd80eb7315ca69e56a3e9719272ae6d622b33ee8f068f897f7aa6cd1`.
The phone had no USB transport and its wireless transport was offline, so the
four Android-native cases were **not run**. Compile-only output explicitly says
`PDF_NATIVE_DEVICE_STATUS=not_run`. Do not equate layout tests with PDF byte or
pixel acceptance.

After compiling current Release classes, run the native script through the
standard logged-command wrapper with `-DeviceSerial <online-device>` (without
`-CompileOnly`), then inspect its synthetic `sample.pdf` and `preview.png`.

Phone production-menu selection, saved-file reading and external PDF-viewer
acceptance remain deferred to the grouped APK round. Do not inherit the Word
1718 device result or mark this extension production-verified from compilation.

## Android Device Evidence, 2026-09-14

`writing-grouped-native-tests-20260914-132807-441` compiled current Release
Kotlin/Java and passed 51 focused tests, zero failures/errors/skips, in 315.2 s.
The source baseline was `6cf138510`; this includes temporary-writing route and
receipt tests, PDF layout, Word, local edit history and native block contracts.
No APK was assembled, published or installed in this acceptance batch.

The first Android 16 shell-runner attempt
`writing-pdf-device-native-20260914-133338-584` aborted with an uninitialized
default typeface, before PDF acceptance. Unlike an app process forked from
Zygote, this `app_process` runner had not loaded its system font map. The runner
now calls `Typeface.loadPreinstalledSystemFontMap` only when `DEFAULT` is null,
then requires an initialized default. This is test-only reflection, never APK
code. The initialization and native assertion are documented in AOSP
[Typeface.java](https://android.googlesource.com/platform/frameworks/base.git/+/master/graphics/java/android/graphics/Typeface.java)
and [Typeface.cpp](https://android.googlesource.com/platform/frameworks/base/+/master/libs/hwui/hwui/Typeface.cpp).

`writing-pdf-device-native-fixed-20260914-133524-917` passed all four cases on
Xiaomi Android 16 in 17.4 s. Payload SHA-256:
`3dd8198ac03fedc01f7695360c6bb8d5a9a3d6633aa3368b0016eb3899a7f275`.
Actual `PdfRenderer` checks accepted seven sample pages, nonblank body pixels,
portrait/landscape A4, an empty one-page document and tall-cell continuation.
The first-page PNG was visually inspected; headings, lists, code, Chinese and
table cells were visible without overlap. Independent `pypdf` extraction found
all 160 numbered paragraphs exactly once and in order.

Exact Unicode extraction **did not pass**: the rendered U+6587 character maps
to compatibility character U+2F42 in this device's PDF text, and `fi` may become
the U+FB01 ligature. NFKC normalization recovers the two expected Chinese strings,
but that is not code-point identity or a fix. Treat PDF as a validated visual
copy, not an exact plain-text round trip. Exact-copy workflows retain Markdown,
TXT and source-file exports. The production menu and external reader still need
acceptance; no complete/default promotion is inferred from this runner.

Synthetic evidence is retained outside worktrees under the Git common directory
at `ai-research-artifacts/writing-pdf-native-20260914-133524/`: `sample.pdf`
(61,236 bytes), SHA-256
`42d35a3e9748d3609f25106b55b4743371e5130d8161214577f34664c11ed8d5`, and
`preview.png`. No private conversation, draft, login or app state was modified.
