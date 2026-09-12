# Message image original download integration

Date: 2026-09-12. Evidence report, not a claim that all image renderers or the
private-native Goal are complete.

## Scope and source

Capability: `android_chatgpt_private_message_image_original_v1`.
Implementation: native viewer wiring and the observed uploaded-image renderer
contract are implemented. Generated-image renderer coverage remains open.
Delivery: grouped APK release/phone acceptance pending; source batch only.

Production `ChatGptSocialImageContentController` previously opened
`ChatImageViewer` without its existing `onDownloadOriginal` callback. Gallery
original download already worked and is reused, not replaced. The compressed
JPEG preview is never saved under the claim that it is the original.

Inspected the current public composer bundle:
[8b34dbc2-fqgb3eqijpn96umi.js](https://chatgpt.com/cdn/assets/8b34dbc2-fqgb3eqijpn96umi.js).
SHA-256: `1d0b132fe9b13120370395bbfbe4bf3324c4dc1213b80e591cd3c6a6db7d16a1`.
Its `RW` renderer pairs each image pointer with matching attachment metadata.
`Den` receives `asset`, `imageName`, `libraryFileId`, and
`checkContextScopesForConversationId`. Its download callback supplies the file
ID, filename, conversation scope and optional library identity. The new adapter
recognizes only this observed runtime/committed owner contract, never executes
the component, and does not infer a file ID from a thumbnail URL.

## Implementation

- `chatgpt_web_private_message_image.js` is the bounded descriptor reader. It
  reuses the committed React owner resolver and existing image-pointer parser.
  A matching conversation and non-optimistic message image are required.
- `chatgpt_web_private_file_download.js` v36 registers a live-bound opaque
  selection. The existing private authorization, library metadata validation,
  original byte download, native lease, progress, save and cancel remain owners.
- Original descriptors are bound to account, document, conversation and selected
  image identity. A changed/unmounted image cannot enqueue a stale response.
  Refreshing a conversation's file index no longer invalidates these independent
  image selections.
- The descriptor is not a signed URL: it survives idle time while its owner is
  still current. Every download still obtains fresh authorization and the native
  one-use lease. Rotating a preview signature does not expire the original file
  identity. The page's existing bounded registry and disposal remain in force.
- `WebChatImageOriginal` validates the native descriptor. Message parsing/mapping
  delivers it to the production image viewer. `ChatGptWebImageSession` shares
  its download dispatch/dialog with the gallery. No second downloader or login
  store was added. Snapshot persistence deliberately omits this live selection.
- Image assets v5 and adapter362 wire the production path. No testing UI added.
  Missing runtime evidence leaves preview unchanged; it does not report that the
  website lacks original downloads.

## Verification

The final Node batch passed 47 cases across message-image registration, gallery
originals, conversation image download, normal file download, image previews and
preview cancellation. New cases cover:

- no request while merely describing/opening a preview;
- unchanged snapshots reuse the same handle;
- exact image pointer, filename and conversation/library scope;
- account, document, route, renderer and image replacement invalidation;
- stale authorization cannot enqueue after image replacement;
- file-index refresh, 15-minute synthetic idle and rotating preview signatures;
- native original byte saving without exporting preview pixels.

Android Release main/test compilation passed. JUnit XML confirms six
`WebChatImageOriginalTest` and eleven `ChatGptFriendMessageMapperTest` cases,
zero failures/errors. These cover production mapping with a non-ready composer
and omission of live download selections from restart caches. The initial
visibility declaration and a missing test-fixture constructor argument were
corrected before the successful run (352.6 seconds). All 125 production adapter
assets also passed JavaScript syntax parsing.

Wireless ADB was rechecked successfully on the existing Xiaomi connection. The
phone still has the previous APK: this new button/descriptor path has not been
installed or visually accepted. No conversation was sent, original user file
modified, Cookie cleared, proxy changed, or microphone opened in this batch.

## Remaining boundary

This reader currently depends on the selected message's committed website
renderer to obtain its real descriptor. The download itself is the existing
private/native route. Do not call this a wholly DOM-independent image index.
Other generated/shared/mounted/watermarked renderer contracts are not inferred.
The previous owned generated-image sample had native previews but an empty
private file index; its cause was not established. It is not proof that generated
images are absent or unsupported. Reuse gallery-original acceptance; next work
should resolve that exact generated-message mapping, then perform one grouped
native-viewer acceptance. Do not repeat accepted voice, dictation or gallery work.
