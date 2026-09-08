# Library files in the current composer

Capability: `android_chatgpt_private_library_attachment_v1`.
Status: completed and production-enabled for the ordinary single-file scope.
Normal 1.1.1588 passed native selection, private ID/name preservation, one composer
card, removal, explicit message send and server-side conversation-file association.
Do not repeat those cases without current regression evidence; unconfirmed
variants below retain their own acceptance boundary.
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

### 1584 Native Association And UI Gap

On 2026-09-09, clicking the real native file menu's attach action returned the
exact successful receipt and one ready attachment. The library automatically
closed; current conversation, draft and message count remained unchanged.
Log: `library-native-association-1584-20260909-032240-607`.

This was not a full composer-UI pass: `WebChatConsumerState` omitted the official
snapshot's attachments, and the production input only mounted local-file
previews. The file reference therefore had no native card/removal control.
The new typed attachment projection and `WebChatComposerAttachmentStrip` reuse
the existing pending-attachment host and `chatgpt_remove_attachment` command.
They perform no upload, message send or navigation. Cards await observed removal,
bind actions to the current page/provider and stop bounded observation on detach.
Work-mode local previews keep their existing owner.

Subsequent 1584 snapshots reduced this staged reference to a generic `附件 1`
DOM-derived entry. That association receipt alone did not establish the private
ID/name retention cause or prove private send-lease/metadata preservation.
The later 1588 acceptance below closes that ordinary-file gap. The test reference was
removed through the existing command afterward, restoring an empty composer
without changing its draft or conversation. No user library files were renamed
or deleted. See the [browser evidence](chatgpt-private-library-browser.md).

### 1586 Composer UI And Private-ID Filter

Normal 1586 passed actual native file-row click, attach, card display and card
removal. Attachment count returned to zero and conversation/draft/messages were
preserved. Names and private IDs were still missing. Logs:
`library-native-menu-1586-20260909-041110-544` and
`library-composer-ui-1586-20260909-041206-952`.

The Android snapshot parser still accepted only `attachment_[a-z0-9]{1,48}`;
it rejected both already-produced `private_attachment_<UUID>` upload IDs and
`private_attachment_mcp_<request>` library IDs. The parser now accepts these
exact bounded forms and rejects oversized IDs without truncation. Composer/send
module 19 uses the verified file store as the display source when every stored
file is privately owned; generic DOM badges cannot create duplicate cards.
Mixed ownership retains unrelated official entries and the existing send guards.

A separate synthetic probe showed `attachedNow()` discards its private owner
after a transient disconnected input even if the same ready store later returns
(`1 -> 0 -> 0` projected entries, official ready entries `1`). That is not proof
of the live generic-label cause: parser rejection already explains lost private
IDs. This batch does not relax identity/store ownership to address that probe.

### 1588 Production Acceptance

On 2026-09-09, normal Release 1.1.1588 (1588), source
`9cf9388f77de6b6b22305045034c0b7800d4bdcb`, was published and installed using the
ordinary replacement update. The local APK and online manifest SHA-256 both equal
`2b0ce969710e1e608e04c2230cd15bcf3886872ac7ec61102b98c0445d81cb22`.
Research/debug transport remained disabled. Cookies and app data were retained.

- `private-attachment-projection-checks-20260909-042622-099`: 49 focused Node
  tests and 30 Android protocol JVM tests passed. The JVM result was checked
  from JUnit XML with zero failures/errors/skips, not inferred from wrapper exit.
- `library-private-ui-final-20260909-043735-439`: actual native library/file-row
  selection and attach produced one ready card with the exact filename and
  private ID. The native removal button returned a successful canonical receipt;
  the card disappeared and conversation, draft and message count were preserved.
- `library-native-send-1588-20260909-044630-530`: in an isolated ordinary chat,
  the same native file menu attached the existing 78-byte fixture. MCP then invoked
  the production input's `set_input_text` and `send_input` handlers exactly once,
  not an official-page send control. One matching user message and an assistant
  response appeared, staged attachments were consumed, and the private
  conversation-file index confirmed the exact fixture in the new conversation.
  Send dispatch through response/file-index verification took 11.4 seconds;
  this is a single functional sample, not a first-token latency benchmark.

The original blank native conversation page was restored with its draft and
message count unchanged. Only the isolated test chat was created. The existing
library file was not renamed, deleted, downloaded again or uploaded again.
The assistant's response alone is not treated as proof of file association;
the server file-index readback supplies that independent check. Project,
temporary, cloud, multi-file and mixed-file variants remain outside this pass.
