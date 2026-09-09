# Native library download UI acceptance

## Result

Completed scope:
`android_chatgpt_private_shared_library_download_v1:personal_library_rendered_txt_png_pdf`.
Reuse the existing default private download implementation. No new downloader,
provider protocol, APK runtime change or duplicate consumer control was needed.
This scope does not complete all mounted/shared-library sources or crash cleanup.

The production native file-library detail's actual Download button was invoked
through an external semantic accessibility runner, not the MCP download action.
All three cases showed the native saved status, returned a newly owned successful
`download_library_file` receipt with `download_saved`, and created exactly one
matching file in the handset's public Downloads directory. Only pre-existing,
fixed synthetic acceptance fixtures were selected. No new upload or send occurred.

| Case | Saved bytes | Independent validation |
|---|---:|---|
| TXT | 78 | SHA-256 equals the original fixed fixture |
| PNG | 4266 | Library size matches; PNG signature and 512 x 384 IHDR match |
| PDF | 22114 | Library size matches; PDF header and terminal EOF marker present |

Saved hashes, which identify synthetic test bytes only:

- TXT: `75e2ed9bfe5772c9918e552ed07c2c0e689e7039367c81bb6906c63e396fa1f3`.
- PNG: `804e80baba505aec57193a5c80a73ee13052e914e99a82110fd989f659d195aa`.
- PDF: `f8cb4add33d9bc3fd410b0203b73ebfacc04893189652af42e5c6d1127fbb818`.

Original-upload hashes were not available for PNG/PDF; their saved hashes are
recorded evidence, not an assertion of byte equality with the original upload.
Header checks are not full image/PDF rendering acceptance.

The native document adapter was current and authenticated while
`composer_ready=false`. File selection, private reads and saving did not require
the broken project composer to recover. The original conversation path, message
count, native/official drafts and non-recording/non-streaming state were restored;
the temporary awake lease was restored too. The three saved fixture files remain
in Downloads, as successful user-facing saves. Temporary host copies were removed.

## Artifact and evidence

- Handset: the pinned Xiaomi 14 Pro; normal installed version `1.1.1612`, code 1612.
- Adapter: 311. Package version was checked before and after this acceptance.
- Online 1612 source: `1d21a4d1b3c43f95c45e084a20cc1f11c584b044`.
- Online APK SHA-256:
  `8e73e9502cd502b5c3747bdcf19b67ad14709b00509a71706a0035dba5bc816b`.
  This task reused the installed package; it did not rebuild, reinstall or
  independently hash the installed APK bytes.
- Logged acceptance: `library-rendered-download-batch-20260909-205640-968`,
  passed in 77.6 seconds. All three case receipts and restoration passed.
- Per-case host-observed duration was 6572 / 7383 / 6913 ms, including external
  runner startup and a separate UI verification step. These are not HTTP,
  first-render or user-perceived download latency measurements.

## Harness correction

The existing native library runner chose its fallback selector immediately after
requesting a feature menu, before the closing drawer had attached that menu.
It could wait on the wrong selector even though the preset library control was
visible afterward. It now waits for either known selector within the same bounded
eight-second window. Assertion reporting also preserves safe ComparisonFailure
codes rather than returning an empty error. An early failed runner attempt was
followed by an observed launcher foreground; no app-crash cause was established.
Reopening from the closed native sidebar with the corrected runner then passed;
conversation/input restoration and the closed-sidebar state also passed.

The external runner now supports Download, saved-state verification, Close, and
exact fixed-fixture search. The reusable smoke script validates new receipts,
actual saved bytes and restoration. Both PowerShell files parsed successfully;
Java compiled against the real Android SDK and ran on the handset. The scripts
are not packaged into the consumer APK. No screenshot or visual-layout pass is
claimed; one bounded semantic tree inspection diagnosed the selector mismatch.
Harness commit: `d2abace84`. The source-size guard passed for all three staged
scripts; this script/document-only delivery intentionally creates no new APK.

## Remaining boundaries

Large-transfer progress, cancel/collapse during transfer, notifications-disabled
behavior, Android process-death pending-row cleanup and Android 8/9 storage have
not been device-accepted by this batch. Project, mounted-provider and conversation
image-pointer downloads remain separate scopes. No source was deleted, no public
share was created, and no Cookie, login, voice or proxy settings were changed.
