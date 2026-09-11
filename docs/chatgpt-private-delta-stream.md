---
version_status: current
reviewed_at: 2026-09-11
---

# ChatGPT private delta stream

## Delivery boundary

`chatgpt_web_private_delta_document.js` v2 is the shared patch owner for the
production response observer, not a new sender. Stream transport v18 and Android
adapter 343 use it through the existing enabled private-stream path. The desktop
bootstrap includes the same asset before its shared policy. Identity, requests,
WebRTC, transcription and account/session ownership are unchanged.

Implementation and synthetic integration are complete. Grouped APK build and
installed-device acceptance are pending. This is not evidence that supplemented
context/PCA file downloads are complete, nor a measured latency/thermal gain.

## Source and semantics

Public asset `conversation-small-ft205i7yqa6zc2nj.js`, SHA-256
`a89952420338983e104e94be5fae8e9ae7a4169b9ea8a4ed30e639641e14f456`,
was rechecked on 2026-09-11. `Aun`/`fun` select the v1 codec at the
`delta_encoding` SSE event. `wun` holds documents per channel and a global
previous delta; `mun` inherits channel/path/op, never value; `hun` expands
compact keys; `_un`/`vun` define relative patches, root paths, array insertion,
append, truncate and deletion. This is source inspection, not a live-account
payload capture. No bundle or credentials are committed here.

The previous observer discarded SSE event labels, kept one compact document,
and patched only `/message/...`. That could lose a channel when another root
arrived, misread explicit-channel object patches as a replacement root, or miss
relative paths and truncations. Strict SSE now decodes the official envelopes
before the existing public-assistant filter and native message projection.
Legacy unlabelled socket envelopes retain their observed batch/continuation
normalization, but use the same document mutation implementation.
Socket traversal stops at consumed numeric-channel envelopes, so a batch's
children cannot append the same text twice. Symbolic `c: "patch"` wrappers still
expose their full-message payload through the existing socket traversal.

Updates copy only changed ancestors. Input values are bounded and detached;
previous emitted snapshots cannot be partially changed by a later failed batch.
Limits include 32 channels, 128 operations per frame, 32 path segments,
4096 array entries, 2 Mi-character strings and an 8 Mi-unit cumulative input
budget. Prototype keys, invalid operations and excessive sizes are rejected.
Unknown encodings, malformed delta JSON and missing markers fail closed. The
observer retains received text, records the existing `error` outcome plus
`delta/decode_error`, and does not report successful completion or replay a send.
The existing official snapshot path remains available to settle the UI.

## Verification

Pure extraction reduced the stream policy from 778 to 666 lines and preserved
six existing script suites. An additional desktop finance test was unavailable
because this worktree has no `pc-frontend/node_modules/typescript`; no dependency
installation or desktop build was performed for this Android batch.

New tests first failed on the old owner (`private-delta-v1-red-20260911-145944-445`).
They cover channel interleaving, inherited fields, relative/nested patches,
source and mask channels, deletion, immutable snapshots, bounded rejection,
chunked SSE and unknown protocol versions. The production transport fixture
checks native message text and completion, failure retention, unchanged official
response ownership, no request-body/header reads, and exactly one official send.
An existing 600k-character response fixture caught an overly small new seed
limit; the full bounded root is retained with the existing native display cap.

The duplicate socket traversal and post-unknown-version delivery also failed
before correction (`private-delta-ownership-red-20260911-151326-618`). Final
`private-delta-release-source-20260911-151507-490` passes 102 runner cases plus
the history script's 12 internal assertions, with zero failures/skips/cancellations.
All 115 Android assets and the combined bundle parse; desktop shared dependency
ordering and scoped Rust formatting pass. None of these is a native compilation.
ADB currently lists no USB device; the previous wireless endpoint times out.

See the source tests `scripts/test-chatgpt-web-private-delta-document.js` and
`scripts/test-chatgpt-web-private-stream-transport.js`. These are synthetic
network/native fixtures, not phone audio or installed-APK evidence.

## Remaining context-source work

`l5i` uses the same decoder for the same-origin context-source supplement stream.
It also owns metadata-status admission, source identity replacement, PCA masks,
per-conversation/message cancellation and completion. Those semantics still
need integration into the bounded file inventory before broadening its scope.
Do not just remove the PCA guard, fetch every historical message, or treat a
cloud source URL as downloadable bytes. Reuse the verified scope/download owners
and the [inline citation boundary](chatgpt-private-file-citations.md#inline-context-file-coverage).
