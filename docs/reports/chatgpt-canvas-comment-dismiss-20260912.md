# Original Canvas comment dismissal

Date: 2026-09-12. Evidence report, not a replacement capability register.
Parent: `android_chatgpt_private_canvas_original_edit_v1`.

## Status

- Implemented: version-bound original-comment dismissal in the existing private
  provider and production native Canvas comment menu.
- Verified: 285 Node contracts and 42 Android release unit tests; release Kotlin
  compilation succeeded. No skipped or failed tests in these runs.
- Not verified: a real original-comment DELETE from the installed production UI.
  This batch did not assemble, publish, or install an APK. Group it with the
  remaining Canvas changes; do not label the parent capability completed.
- Reused: the original document reader, edit-context observer, confirmation,
  selected-document ticket, single-flight write owner, and readback verifier.

## Official protocol evidence

Public asset `conversation-small-h1dtzoris1y9588z.js`, SHA-256
`da08c64c132306779e09ba89cac64fa560b120e7560ffdc29b3ce5a0b8ccd67e`:
`kir` performs `safeDelete` on
`/textdoc/{textdoc_id}/{version}/comment/{comment_id}`, with query `reason` and
returns the response `version`. The reason enum separates `accept` and `dismiss`.
The native provider uses only `reason=dismiss`, with no request body.

Public editor asset `ac476d6c-foq8estmw3bbr7op.js`, SHA-256
`f341cbd1ebc80c14838f6bada429d1826a5a78577f094f5be452ca7663c3c895`:
the dismissal callback invokes the comment owner with `DISMISS`; the acceptance
callback first removes with `ACCEPT`, then creates a generated editing turn.
Those are distinct workflows. Neither ordinary body save nor comment dismissal
implements acceptance or AI rewriting.

Only public code was inspected. No account headers, tokens, or private document
content are recorded here.

## Behavior and ownership

1. Opening the native comment detail exposes a confirmed dismissal action.
   Cancel performs no write.
2. Before DELETE, re-read the canonical original and require an unchanged
   document, account/session, ticket, version, and edit context. Reject unsafe or
   missing comment IDs, streaming, and pending official edits.
3. Submit one version-bound DELETE. Read back the original and require a newer
   matching version, unchanged title/type/body, and exactly the other comments.
   HTTP success alone does not confirm the operation.
4. Any uncertain dispatched write blocks further document writes. Verification
   only reads; it never repeats DELETE, falls back to DOM, or submits a body save.
5. On confirmed readback, update the native base and remove only the requested
   comment from the unsaved draft. Retain edited body text, other comments and
   their unresolved anchors. Rebase/adopt clears stale dismissal intent.

The unexported official optimistic-comment store is not claimed as observed.
Fresh canonical comparison and the server version are the conflict guards for
that path. Real-device acceptance must still check concurrent official edits.

## Checks

- `canvas-comment-dismiss-combined-20260912-090044-773`: 285 passed, zero failures
  or skips across dismissal, original documents/policy/actions, rename, history,
  sharing, edit observer, and existing Canvas content/publish/share contracts.
- `canvas-comment-dismiss-release-tests-20260912-090120-452`: `BUILD SUCCESSFUL`;
  JUnit XML: draft 13, documents 7, management 6, content 6, readiness 10.
- Cases include Unicode comment ranges; unchanged native drafts; explicit intent;
  foreign or stale selections; pending edits; account changes; readback mismatch;
  missing/old response versions; response loss; and concurrent write rejection.
- The first local combined run lacked the existing optional AST parser path.
  After supplying the parser and reviewed public-asset paths, all contracts ran
  without skipping. This setup error did not require a runtime-code change.

## Next verified source leads

These are research locations, not shipped features or device acceptance:

- `d3304073-k8khdx5ezvb9oyu8.js`, SHA-256
  `4c59db7c2db5afb92e5b037b891fe28a99bc8ab185170c737aad9587a7d5b559`:
  exported `a` (`Hn`, initialized by `Wn`) owns Canvas generated turns. It uses
  `request_completion.canvas.textdoc.use_create_textdoc_turn.1`, exact Canvas
  document/version/selection metadata, and an appended contextual message.
  Reuse this owner only after checking its imports and current runtime bindings;
  do not imitate it with a plain-text send or a comment DELETE.
- `bc86e6a9-lyyfjzfq9uy5wnj2.js`, SHA-256
  `9417a623b117d5c76343b91f9a2c6c3a0402c24d50cec8fd8a671e2fc1493eeb`:
  exported `n` (`Xe`, initialized by `Ze`) posts document exports to
  `/export_doc/canvas` with `textdoc_id` and `export_type` (`pdf` or `docx`),
  then consumes a binary response. Markdown and code exports use local content.
  Its export `i` is copy telemetry, not the generated-turn owner.

Next acceptance should cover original save/history restore/first share/rename
and dismissal together, preserving the original conversation and native draft.
The [remaining work list](../web-ai-private-native-remaining-batch.md) remains
the scope index; Google and thermal work do not precede this ChatGPT gate.
