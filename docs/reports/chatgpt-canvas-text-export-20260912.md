# Original Canvas Text Exports

Status: implemented candidate; production-phone acceptance pending. The parent
`android_chatgpt_private_canvas_original_edit_v1` remains partial. This closes
the Markdown/source export code gap after [comment acceptance](chatgpt-canvas-comment-accept-20260912.md)
and [PDF/Word export](chatgpt-canvas-original-export-20260912.md); it does not
reimplement the download owner or replace original files with rendered previews.

## Provider Evidence

The reviewed `web_20260912` export module has two distinct local paths:

- Code uses a Blob of the original content, with the explicit `Z` language,
  extension and MIME table. All 37 reviewed code types are mapped, including
  React/JSX and Dockerfile; unknown types and `loading` are not guessed as TXT.
- Markdown uses `G(content)` without `preserveContentReferences`. With no
  `contentReference` or `hiveTranscript` marker, it preserves the original text
  verbatim. Otherwise it imports `unified`, applies the official hive plugin,
  directive-strip plugin and `CANVAS_REMARK_PLUGINS` in that order, processes
  the content, then trims the serialized result. `G` is not a public export;
  calling an inferred alias would not be valid.

| Public source | SHA-256 |
|---|---|
| `bc86e6a9-lyyfjzfq9uy5wnj2.js` | `9417a623b117d5c76343b91f9a2c6c3a0402c24d50cec8fd8a671e2fc1493eeb` |
| `conversation-small-h1dtzoris1y9588z.js` | `da08c64c132306779e09ba89cac64fa560b120e7560ffdc29b3ce5a0b8ccd67e` |
| `4813494d-gf2h57w5fiay19bd.js` | `6c015001732054f4143ef1922609407c540967762109dcd128bbf56706889c3e` |
| `ac827dee-b2d6yzv273npzjvz.js` | `1124a81c8fb3dcc4042b33666f47af733fe9e0dd2d8a9dd607a1f0bccee34d4d` |
| `e5d54aa7-o5mtxxnk4j8zox9y.js` | `393bf8650d692d62ffdf1d5522b71d49bd09a783b81046112c911bf09f4c82d0` |
| `1c4de3ec-ix5n1yyu8whxeib8.js` | `7f5583294c72209a9bb06eb870cc021bc52862606e9f083e48a0048f65007296` |
| `6afb0137-dqge0sx8jli56ai8.js` | `33dfe354d1cd48931719cdd38e93aff36ecfead535e6a4b7c812e7f33f59d177` |

The source-contract test checks hashes, AST definitions, the exact type table,
imported module names/initializers and the official call sequence. It evaluates
only the pinned enum/table expressions in an isolated Node context, not the
full provider application or an authenticated page. No private document content,
request credentials or vendor code copies are committed.

## Native Behavior

- The production original editor offers Markdown/PDF/Word for documents and the
  evidenced source format for code. The existing dirty-draft guard requires an
  explicit save first. No separate test UI or new download screen is introduced.
- Export commands carry the selected canonical document and `md` or `source`,
  not caller-provided body, URL, extension, headers or arbitrary serializer.
- Existing canonical pre/post checks cover the original body, version, title,
  comments, identity, document token, route, streaming and official pending edits.
  Competing writes retain the same owner while preparation is active.
- Plain Markdown and all source exports avoid plugin loads. Special Markdown
  uses exact version-gated official modules cached in the current page, not DOM
  menu/composer readiness. It does not fall back to destructive regex stripping.
  Imports and asynchronous conversion each have a 1.5-second deadline. A stalled
  converter releases Canvas ownership; a late result cannot save a file. Text
  remains subject to the existing 128-KiB UTF-16 content policy, never truncation.
- Text exports create local UTF-8 Blob/Response bytes and use the existing
  acknowledged begin/chunk/commit transfer. They make no export POST, body save,
  completion request, system-print call or synthetic file URL. PDF/DOCX retain
  their separate existing private POST.
- HTML and JSON source files are explicit attachments. Generic downloaded HTML
  error-page protections remain intact. Nothing is executed as part of export.
- Android and the page independently validate the format/MIME/extension tuple.
  Names exclude controls, directional overrides and path separators, and stay
  within the native 150-character limit without truncating long extensions.
- Cancellation, stale identity and version changes prevent final save. The
  existing one-shot handle and lost-commit acknowledgement behavior are reused;
  a possibly saved file is never automatically saved again.

## Verification And Delivery

- `canvas-text-download-final-20260912-111710-917`: 615 Node tests passed,
  no failures/skips. Includes all original Canvas and shared-download tests,
  real-source contracts, strict native actions and unchanged PDF/DOCX behavior.
- New cases cover all source formats, BOM/CRLF/Unicode/whitespace, empty content,
  multichunk transfer, exact Markdown plugin order/cache, timeout and incompatible
  modules, account/page/version changes, cancellation, malformed output and
  independent Android metadata parity. Special-plugin execution is mocked;
  AST/hash checks are evidence of the official recipe, not live rendering proof.
- `canvas-text-export-android-20260912-110911-566`: Android Release compilation
  and 36 JUnit tests passed (export 5, original documents 7, management 6,
  download policy 5, native draft 13), zero failures/errors/skips. Duration
  313.6 seconds, no timeout or stall. Subsequent changes are limited to the
  page serializer deadline, its Node checks and documentation, not Kotlin code.
- No APK assembled, published or installed in this batch. Grouped phone
  acceptance must confirm native menu/download interaction and real provider
  Markdown serialization of directive-bearing documents, together with the
  original-editor save/history/share/AI/comment cases already listed upstream.

## Next Work

Do not repeat this format-mapping implementation. Run the grouped original-editor
acceptance, fix concrete failures, and maintain the remaining special-source and
Google gates in the [remaining map](../web-ai-private-native-remaining-batch.md).
Existing verified audio/subtitles/dictation/read-aloud are outside this change.
