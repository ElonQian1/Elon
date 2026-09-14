# Writing Block Word export: Android Unicode correction

## Observed failure

`writing-docx-production-ui-1717-20260914-083410-094` reached the production
native editor, edited a controlled existing Writing Block, exercised undo/redo,
selected DOCX and created a real file on the Xiaomi phone. It failed the strict
XML check after downloading that synthetic export. This was not a pass: the
Android Transformer emitted a supplementary character as two invalid XML
surrogate references (`&#55357;&#56832;`) rather than one scalar value.
The earlier desktop JVM/Word checks did not establish Android compatibility.

The failure involved no sent chat messages or cloud writes. The fixture was
retained; the first attempt's editor cleanup was unconfirmed, while the outer
lookup restored the original conversation. No private content or credentials
are retained in this report.

## Correction

`WebChatTextBlockXml` serializes only the generated OOXML DOM using the standard
XML Pull serializer. It preserves namespaced attributes and XML escaping and
does not rewrite serialized XML with regexes or replace Unicode characters.
Unexpected node types, namespaces and excessive depth are rejected.

`WebChatTextBlockDocx` keeps the existing CommonMark conversion, ZIP parts,
styles, lists, tables and local file publication. Markdown, text and source-code
exports, cloud-save requests and the native editor remain unchanged.

The existing DOCX tests now run with the repository's Android 34 Robolectric
runtime rather than desktop XML serialization. Supplementary code points,
including the Unicode upper boundary, must round-trip without surrogate numeric
references. The acceptance tool reports the bounded `docx_xml_invalid` reason
and separately tests valid scalar references and the observed invalid pair.

## Evidence Before APK Recheck

- Android runtime probe: byte stream, UTF-8 Writer and StringWriter variants of
  the old Transformer all reproduced the invalid pair. XML Pull emitted valid
  scalar references for supplementary characters in text and attributes.
- The newly compiled production encoder was run directly in the phone runtime,
  reusing the installed APK's unchanged dependencies. Its 3,140-byte DOCX passed
  the independent ZIP/XML validator: 10 exact paragraphs, 1 table, 2 list items,
  matching Unicode, tabs and code indentation. SHA-256:
  `2941e4f7b85f6388ec8b640c295f7f3c5958056e635efd843a1b11d07c97311b`.
- Microsoft Word opened that Android-generated file read-only: 12 Word
  paragraphs and 1 table. Word includes its paragraph markers in this count;
  the document-body validator checks the expected 10 paragraphs independently.
- `writing-docx-surrogate-evidence-20260914-084440-166`: 61 offline checks passed.
- `writing-docx-android-runtime-regression-20260914-084738-254`: 22 tests passed,
  including 13 Android-runtime DOCX tests, 0 failures/errors/skips, 129.4s.
  Release Kotlin/Java compilation passed. The first test run used a plain JVM
  and failed on Android's unmocked XML factory, not on document content; the
  corrected test runtime reuses existing project dependencies.

The encoder probe is not the final APK UI recheck. Record that separately after
the corrected package is installed; do not relabel the 1717 failure as success.
