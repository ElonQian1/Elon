# Library files in the current composer

Capability: `android_chatgpt_private_library_attachment_v1`.
Status: implemented, offline verified and published/installed in 1.1.1581;
live composer association and explicit-send acceptance remain pending.
This extends the [native library browser](chatgpt-private-library-browser.md).
It is not a second uploader, an independent message POST, or a new speech path.

## Evidence and scope

The observed public composer asset is `8b34dbc2-nhot65scqrg20d6p.js`, SHA-256
`36644eb82aac9c399bce384c18140f8c878dd780c8f787440b80f27971729733`.
Its recent-file picker calls `onAttachLibraryFile` with the backing `file_id`,
library ID, name, MIME, byte size, native provider and composer-picker entrypoint.
`SB.attachLibraryFile` builds a metadata-only File, a ready entry and a fileSpec,
then appends it to the existing FilePicker store. Images use 512x512 reference
metadata there; this is not a claim that the actual image has those dimensions.
No byte download or upload is required merely to attach the existing reference.
Public source evidence is not a live-server acceptance result.

Initial supported scope: one ordinary native-library text/document or JPEG,
PNG, WebP image, up to the existing 8 MiB attachment safety limit, in an empty
ordinary chat composer with the current library option enabled. An existing
conversation must pass the existing private context reader. Temporary chats,
project chats (including project membership discovered on an ordinary route),
custom GPTs, cloud mounts, saved entities, artifacts, trashed files, occupied
composers and unrecognized metadata remain unconfirmed, not fabricated support.
Multiple simultaneous library selections and mixed local/library batches are
not included. Existing local batch upload is unchanged.

## Ownership and UI

- The library snapshot exports `canAttach` and an opaque handle, never the
  backing ID or identity headers. The page owner selects only fresh observed
  records. A refresh, rename, soft deletion, account/document change or expired
  catalogue invalidates that exact selection before association.
- The new module reuses the existing attachment composer's binding, context
  reader, ready publication/readback, display merge, Remove action and prepared
  send lease. Only its ready-entry builder follows the distinct library shape.
  It does not manufacture a successful upload receipt or mark a library pick
  as automatic upload deduplication.
- Library association and local upload share the existing sender's exclusion
  boundary. Duplicate command IDs share one association receipt. Changing
  conversation/model, replacing the store or cancelling pending attachment work
  prevents a late result from populating another composer.
- Native file actions add "加入当前聊天". The browser stays visible while a
  bounded receipt watcher observes the exact request. Only confirmed association
  returns to the unchanged chat. Unknown/failure stays in the browser; no draft
  clearing, automatic message, automatic page navigation or write replay occurs.
- Closing the browser stops its watcher, not an already accepted association.
  No server-side cancellation is claimed. Existing upload cancellation, identity
  invalidation and file removal retain their existing owners.

## Verification

Focused tests cover the ordinary/image ready shape, metadata-only bytes, existing
send-lease consumption, native display/removal, duplicate command ownership,
selection refresh/mutation invalidation, unsupported variants, conversation and
identity changes, and the typed native command/receipt wiring.
The related Node suites passed 102 cases. Release source/test compilation and
12 focused Android JVM tests also passed. Grouped APK 1.1.1579 was subsequently
published and installed without clearing app data; see the browser's release receipt.

The older library-upload integration fixture lacked the committed root's child
link. Its 11 failures were reproduced with the unchanged HEAD composer/sender.
The fixture now models the committed branch required by the existing production
owner resolver; production branch validation was not relaxed.

Device verification must still establish that the
current official runtime accepts the reference, that a selected file appears in
the production composer and reaches the intended conversation after explicit
send. The user has signed in. The 1579 catalog object-event defect is fixed in
1581: request-bound native reads are verified and nine root rows advertise
attachment handles. This does not verify association itself. The rendered native
menu/browser handoff remains unconfirmed; see the [browser evidence](chatgpt-private-library-browser.md).
No user library files were renamed or deleted.
