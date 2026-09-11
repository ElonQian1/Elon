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

Android publication and native device evidence are pending for this batch.
Do not infer raster attachment/rename/deletion acceptance from the 1666 download
or from ordinary-file mutation acceptance. No original user file may be deleted
to manufacture a test result. The broader Goal remains active; Google stays last.
