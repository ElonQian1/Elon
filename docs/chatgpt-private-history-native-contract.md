# Private history native contract

## Delivery boundary

This repairs the existing `android_chatgpt_private_conversation_prefetch_v1` path,
not a second history transport. The baseline is `680e96e75`; candidate adapter is
`261`, history transport `17`, and image asset worker `2`.

- Code: implemented; targeted JavaScript tests passed.
- Android release-source compile and targeted unit verification: passed, 33 tests.
- APK release: published and installed in `v1.1.1540`; device UI acceptance pending.
- [Grouped delivery evidence](web-ai-private-native-remaining-batch.md#grouped-release)
  records the source/hash and separates installation from protocol acceptance.

## Confirmed gaps

The private history producer emitted `content: "text"` and `parts: []`.
`ChatGptWebProtocol.textContent` accepts a `content` array, and its part parser
also reads that array. Therefore a successful private GET could produce zero
native messages until the official DOM supplied the history. Existing JavaScript
tests checked the producer's string, not the consumer's contract.

History refreshes also supplied empty composer/dictation fields and
`composerReady: false`. Those are not observations about the current composer.
Treating them as full snapshots could downgrade a ready session to loading.

## Implementation

- `chatgpt_web_private_history_projection.js` emits the existing native content
  array. It reuses the private stream citation/finance/chart parsers and retains
  bounded image/file descriptors, including messages with no text.
- Only the selected ancestor chain is shown. A missing current node requires a
  unique leaf; ambiguous, broken, or cyclic mappings defer to the official page.
  Hidden, analysis, and tool-directed records are not displayed as chat answers.
- Private GETs carry `snapshotScope: content`. For the same conversation,
  `ChatGptWebContentSnapshotPolicy` preserves live composer, draft, attachment,
  model, dictation, read-aloud, and access state. Active streaming remains the
  authority. Different conversations never inherit those interaction states.
- Android and desktop bootstrap both load the shared projection. The desktop's
  existing compatibility wrapper already accepts canonical content arrays.
- Image requests coalesce by handle, permit two active jobs and sixteen pending
  jobs, and expire each active job after eight seconds. Timed-out/disposed work
  cannot later publish success. Bitmap/canvas resources are released; streamed
  downloads are bounded before decoding.
- Query-addressed image identities remain distinct while known expiring signing
  fields are excluded. Gallery synchronization no longer marks missing DOM as a
  successful empty library, which previously suppressed refresh for six hours.

No Cookie, login data, proxy code, microphone path, or official send protocol is
changed. File/image descriptors are not a new uploader or download protocol;
unresolved previews and newer rich widgets still need the established page path.

## Verification

`android/app/src/test/resources/webchat/private-history-contract.json` is shared
by the JavaScript producer test and the real Android parser test. Neither side
may independently change the wire format while retaining a passing fixture.

Targeted checks:

- `test-chatgpt-web-private-history-projection.js`: 12 cases.
- `test-chatgpt-web-private-transport.js`: producer integration and snapshot scope.
- `ChatGptPrivateHistoryWireContractTest`: native parsing of the same fixture.
- `ChatGptWebContentSnapshotPolicyTest`: composer/voice preservation and isolation.
- `test-chatgpt-web-image-request-lifecycle.js`: coalescing, bounds, timeout,
  late completion, cancellation, and oversized bodies.
- Existing image assets and private stream policy/transport tests.
- `test-chatgpt-web-image-gallery-readiness.js`: four readiness/cache cases.

The grouped `:app:testReleaseUnitTest` check passed for the history wire contract,
content snapshot policy, and existing web protocol tests. The first fixture test
omitted the bridge envelope; it now uses the production schema/event envelope.
The final run passed in 102.4 seconds. Desktop shared-asset loading and existing
JavaScript rich-content compatibility checks passed; no desktop binary was built.
The repository formatter does not register `desktop-shell` as a known crate and
rejected that path before formatting. Its four-line asset include follows the
adjacent existing entries; desktop formatting/build validation remains deferred.

Combined device acceptance must check a history switch before DOM hydration,
private refresh with an unsent draft, and repeated image opening under a stalled
download. No new real-device latency or temperature improvement is claimed yet.
