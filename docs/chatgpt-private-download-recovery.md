# Private download crash cleanup

Capability candidate: `android_chatgpt_private_download_crash_cleanup_v1`.
Status: **implemented, focused JVM tests and Android SDK type compilation passed;
grouped APK build and actual Android storage acceptance pending**. Not completed
or device-verified. This changes native storage ownership, not the website API,
adapter version, ordinary DownloadManager lane, upload or voice protocols.

## Failure and implementation

The preceding binary sink kept the pending MediaStore URI or `.part` path only
in memory. Normal cancellation removed the resource, but process death skipped
that callback. There was no persistent ownership or startup recovery call.

The binary sink now creates a versioned marker under app-private no-backup
storage **before** allocating external storage. Its filename is only a random
download ID and its contents are empty. It contains no URL, file name, account,
conversation, Cookie, token, audio or attachment bytes. The marker is forced to
disk and held under a per-download OS file lock until publication or discard.
A short journal guard protects creation and cleanup across threads/processes;
external storage I/O does not hold that guard.

At application startup one background worker visits at most 128 known markers.
Active owners remain locked and are skipped; failed cleanup keeps its marker
for a later startup. Unknown files, directories and marker symlinks are ignored.
There is no recurring timer, foreground DOM probe, network request, permission
prompt or request replay.

- Android 10+: pending rows use an opaque reserved display-name prefix. Recovery
  includes pending entries explicitly, verifies the app's owner package and only
  deletes matching `IS_PENDING=1` rows. Publication sets the final display name
  and clears pending state in one update. A completed row is never deleted even
  if the process died before removing its marker.
- Android 8/9: pending bytes use an ID-only `.part` file in a dedicated
  app-external staging directory. Completion moves it without replacing a final
  file. Recovery visits only the exact ID's regular staging file, including
  leftovers after an OS upgrade; it does not scan/remove final downloads.
- Allocation errors leave a recoverable marker. Successful cancellation removes
  it only after discard returns. Cleanup failure cannot change a successfully
  published file into a failed transfer or delete that saved file.

The storage model follows Android's
[pending-media contract](https://developer.android.com/reference/android/provider/MediaStore.MediaColumns#IS_PENDING)
and [pending-query controls](https://developer.android.com/reference/android/provider/MediaStore#QUERY_ARG_MATCH_PENDING).
The new Android provider calls are type-checked, not runtime-verified yet.

## Verification

On 2026-09-07 Kotlin 2.0.21 compiled the real journal, destination ownership,
transfer, download policy and session sources. **31 JUnit cases passed**:
11 journal cases, four ownership cases and 16 existing transfer/policy/session
cases. They include actual temporary files, active ownership, allocation and
discard failure, completed-file preservation and bounded cleanup. A real JVM
child holds the file lock; recovery skips it, then recovers after that child is
forcibly terminated and the OS releases its lock. The initial immediate-after-
exit assertion failed on Windows because the lock was briefly still occupied;
the test now waits up to five seconds for release. Production cleanup still
skips locked files rather than breaking a live lock.

The six storage-related production Kotlin files also compile against the real
Android SDK 34 and AndroidX Core 1.12.0 dependencies. This is not an application
build, lint pass or Android MediaStore execution. Existing private ordinary,
shared-library and connector-copy download suites passed **89 Node cases**,
without changing the website request path or test expectations.

Logs: `download-recovery-junit-final-20260907-123316-541`,
`download-recovery-android-types-20260907-123317-679`, and
`download-recovery-js-20260907-123353-466` under the shared command-log directory.

## Remaining acceptance and limits

- In the grouped production APK, interrupt a synthetic binary download, restart
  the app and verify only its pending bytes/row disappear. Repeat around final
  publication; the completed file and ordinary DownloadManager jobs must remain.
- Check actual Android 8/9 staging and Android 10+ MediaStore behavior, unavailable
  external storage, provider errors and restart during an active second owner.
- This is **not byte-range resume**, a persistent download UI/history, automatic
  re-download or a restored completion receipt. The old page lease expires when
  its document/process dies; the user must explicitly select Download again.
  Range/validator support and resumable website semantics remain unverified.
- Old unjournaled downloads from earlier candidates are deliberately not swept:
  their broad `elon-` name also belongs to the DownloadManager lane. Do not infer
  deletion ownership from that prefix alone.
- No new APK, phone transfer, battery or thermal result is claimed for this batch.
