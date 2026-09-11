# Project Chat Library References

## Gap And Protocol

Normal 1667 accepted personal raster reference/removal and rename in ordinary
conversations. The project attempt stopped at `library_attachment_scope_unconfirmed`.
This report concerns personal library files referenced in a project chat, not
project file collection management, new-file upload, or shared-library grants.

Current inspected public assets are in `runtime-20260911-b`. The conversation
asset SHA-256 is `a89952420338983e104e94be5fae8e9ae7a4169b9ea8a4ed30e639641e14f456`.
Its `aWr` returns project eligibility only after the official project recall
gate and setting pass. `wnr` passes that result to `zun`; the main composer
`M2n`/`e2n` receives `isLibraryEnabled`, `libraryEligibilityReason`,
`isProjectEnabledForGizmo` and `gizmoId`. The native bridge reads those committed
props, not a guessed feature flag or copied eligibility implementation.

`Pvn` receives the project flag, eligibility reason and computed attachment
limits. It calls `DV.validateChatAttachment` before `DV.attachLibraryFile`.
The latter retains the same backing file ID, library ID and artifact type in a
metadata-only ready File. It does not add project-upload metadata or upload the
bytes again. Conversation `K2n` carries those reference fields into submission.

## Implementation

- Attachment composer v23 captures the explicitly eligible project identity
  alongside its existing account/document/model/route binding. Eligibility or
  project changes invalidate pending work and submit leases.
- Library attachment v9 uses the existing composer preparation and branch
  guard. The server conversation/project and current official leaf must agree
  with the captured project. New project chats reuse the project scope reader;
  unavailable or mismatched scope cannot publish an attachment.
- Library limits policy v4 admits the matching project only with the current
  official `eligible` reason. Project first references and appends use the
  existing official size/count validator, without opening its menu.
- Append/removal, mixed local uploads, ready-entry serialization and submit
  acknowledgement keep one existing owner. Local uploads keep their original
  project-write versus chat-only permission handling; references do not acquire
  project file collection write permissions.
- Adapter 351 wires these module versions. No new HTTP endpoint, copied token,
  second file queue, DOM click, upload protocol or voice implementation.

## Verification

The initial focused suite passed 71 cases, including new/existing project
routes, read-only membership, explicit denial/unknown eligibility, mismatched
server scope, branch changes, count rejection, duplicate reference, append,
removal, submit revocation and mixed upload context preservation.

Another 202 existing library append/local-append, runtime attachment submission,
composer, upload-library integration, mounted reference and full bundle cases passed.
These establish code behavior, not live project eligibility or server send
acceptance. Device acceptance and release evidence follow in this report.

Release 1668 (adapter 351, source `10c7449eb`) was installed without clearing
data. Both existing-project and new-project native library references failed
before publication with `library_attachment_context_changed`. The new-project
probe observed a successful project GET (HTTP 200); no attachment, send, upload
or delete occurred. Original conversation, empty draft and awake policy were
restored. Project title slug canonicalization initially confused the acceptance
script; checking the stable project ID removed that separate test precondition.

Composer v24 and project scope v8 now retain closed failure reasons for server
membership, selected branch, project permission and upload policy checks.
Library v10 reports them separately from actual input drift. This does not
weaken admission or replay a write. Targeted compatibility tests passed;
device localization with adapter 352 remains pending.

Capability `android_chatgpt_private_project_library_reference_v1` currently has
`code_status=partial`, `verification_status=failed`; the shipped project path
still needs live fault localization and repair. Google remains last. The wider
Goal remains active, including raster deletion and other unverified sources.
