# Personal Raster Library Actions

## Scope And Evidence

The 1666 native download acceptance established nine personal PNG raster rows
with library/backing-file identities and no cloud, project or saved-entity flags.
Their artifact type classifies as `other`, not `image_gen`. Existing attachment
and mutation owners rejected every non-null artifact marker, independently of
the accepted downloader. This was a missing capability filter, not a missing API.

Current public runtime evidence is retained in `runtime-20260911-b` under the
shared Git research-artifact directory. Conversation asset SHA-256 is
`a89952420338983e104e94be5fae8e9ae7a4169b9ea8a4ed30e639641e14f456`.
The source's `Uqn` preserves library/backing IDs and artifact classifiers.
`Gwr.rename` uses the library-file PATCH with `file_name`. `Gwr.remove` delegates
soft deletion to `Lwr` and requires `file.deletion.completed`; progress or HTTP
success alone is not completion. Existing native mutation routes already match.

The composer source's `DV.attachLibraryFile` uses a metadata-only File and
retains `libraryArtifactType`, library ID and backing ID in its ready entry.
Conversation `K2n` includes that type as `library_artifact_type` during outgoing
serialization. Reusing a backing image without this provenance is not equivalent.
No endpoint, thumbnail-based original URL or undocumented mutation was guessed.

## Implementation

- `chatgpt_web_private_library_raster_policy.js`: one pure source predicate,
  extracted without changing the accepted downloader's behavior.
- `chatgpt_web_private_library_attachment.js`: admits the same personal raster
  sources within the existing attachment size/name constraints and preserves
  the artifact type through the existing composer/submit owner. No binary
  download, reupload, automatic send or second attachment queue.
- `chatgpt_web_private_library_mutations.js`: admits those sources to the
  existing rename and soft-delete owner. Existing explicit confirmation,
  account/document/selection binding, single-flight receipts, completion
  acknowledgement and uncertain-write cooldown remain authoritative.
- Adapter 350 loads the shared predicate before all three operation owners.
  Ordinary files, mounted sources and special document artifacts keep their
  existing operation-specific behavior; a raster marker does not grant their
  permissions or substitute for identity/scope checks.

## Verification

The extraction passed its original library/download contracts. The combined
action tests passed 162 cases, including artifact marker retention, native
removal revoking a submit lease, ownership/format/size exclusions, rename cache
retention, exact soft-delete identity, completed-event acknowledgement and no
write replay after uncertain outcomes. Existing append/selection-refresh and
deadline tests remain included.

One new test initially assumed a submit lease retained the same JavaScript
object identity. The existing sender intentionally snapshots entries. The test
was corrected to verify preserved values and revocation, without changing that
existing ownership behavior.

The combined bundle/download contracts passed another 130 cases, including the
current complete adapter bundle. Normal Android Release assembly passed in
5m44s; publication and installation completed in 388.7s. These are build/run
durations, not interaction latency measurements.

## Normal 1667 Acceptance

- Source: `1515599e7f63ac6b5da1615d7c2fdb13c799e76c`; adapter 350.
- Release: `v1.1.1667 (1667)`, installed on the connected Xiaomi via `adb install -r`.
- APK SHA-256: `bd64aa61352cf34c3efdcdebbd0708b1b7bb2f554e40eb04845915014d0caecb`.
- Native library Refresh returned 21 rows, including nine personal raster
  artifacts. Their download/attach/rename/trash controls were available.
- In an ordinary blank conversation, a previously identified 662,362-byte PNG
  was selected through the native library UI. Its ready attachment appeared in
  the native input; native removal cleared it. No send or reupload occurred.
- The same artifact was renamed through the native rename dialog. A fresh
  server catalog confirmed the temporary name; the existing mutation handler
  restored the original name, with a second fresh catalog confirming restoration.
- Original project conversation, empty draft and awake setting were restored.
  No microphone use, message sends, uploads, deletes or private-content export.

Capability `android_chatgpt_private_library_raster_attachment_v1` is
`completed`, `device_verified`, default-enabled for ordinary-conversation
reference/removal. Capability `android_chatgpt_private_library_raster_rename_v1`
is `completed`, `device_verified`, default-enabled for personal raster rename.
Reuse these scopes; do not infer message submission from attaching alone.

The bounded native acceptance took 96.4s and recorded `passed=true`,
`native_attach=true`, `native_remove=true`, `native_rename=true`,
`restored_name=true`, `restored_context=true`, `awake_restored=true`.
Evidence log: `library-raster-actions-ordinary-device-20260911-20260911-193710-587`.

## Remaining Boundaries

Raster soft deletion has implementation and offline completion/uncertain-write
tests, but no raster deletion device acceptance. Never delete an original user
file to manufacture a result; use a genuinely disposable owned sample.

The first attachment attempt in the original project conversation was rejected
with `library_attachment_scope_unconfirmed`, before modifying attachments.
Existing `captureLibrary`/attachment guards reject all project conversations.
Current official composer `wnr` passes `aWr` eligibility into `zun` as
`isLibraryEnabled`, `libraryEligibilityReason` and project identity. `aWr`
conditionally permits project recall when its official gate and recall setting
allow it; otherwise it reports `project_recall_gate_disabled` or
`project_recall_disabled`. Projects are not universally unsupported.

Next work must bind that observed eligibility to the existing project/composer
owner, without forcing flags or relaxing identity checks. Previously accepted
project new-file upload/send is a separate capability, not proof of library
reference support. The broader Goal remains active; Google stays last.
