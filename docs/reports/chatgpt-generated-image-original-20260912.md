# Generated Message Image Originals

Date: 2026-09-12. Source evidence plus normal 1684 device acceptance.

## Scope

`android_chatgpt_generated_message_original_v1`:

- `code_status=completed` for the observed inline final-image renderer.
- `verification_status=device_verified`, default-enabled; [normal 1684](chatgpt-image-files-1684.md)
  passed the actual native bubble/viewer/original-download path and saved PNG decode.
- Download uses the existing native image viewer, file authorization, save/cancel
  owner and opaque descriptor introduced in
  [message originals](chatgpt-message-image-original-20260912.md).
- This is private HTTP original download plus a committed page-runtime descriptor
  reader. It does not eliminate the identity WebView or independently render all
  generated images from history HTTP responses.

## Observed Contract

Public bundles retrieved from `https://chatgpt.com/cdn/assets/`, without account
headers or private message content:

| Bundle | SHA-256 |
|---|---|
| `c1811d6e-jnjixufzpxcwv4zx.js` | `fa25fcc26af86e6487f711d1bbdcac2019c077c9029fdbd7ad542397be6a4fe4` |
| `ecab41d6-l7yjx7xz4auxotb7.js` | `e6a06afac28c748b7db6dca89ae96485c1653f5d25feb9e391152550aaddf923` |

The existing composer imports `ImageGenMessage` separately from ordinary
`TextMessageImageGroup`/`Den`. Its committed `lo` owner passes the generated
pointer, message and conversation to `qa` (image) and `Xt` (inline actions).
The inline action appears after final rendering, not for compact/preliminary
images. The generated branch need not have the ordinary text-message DOM role.

`Xt` defaults to the watermarked pointer when `hasWatermarkedDownload` is true;
the component receives that policy from the actual account state. A paid account
is not forced to use a watermark merely because metadata contains one.
`j$e` in the conversation bundle and `PDt` in the shared bundle resolve the asset
with `conversation_id`, `inline=false` and `download_intent=true`. This differs
from the existing uploaded-image context-scope query. A project conversation is
not evidence that the generated file itself has project ownership.

## Implementation

New `chatgpt_web_private_generated_image.js` reads only a proven committed image
subtree, bounded to 384 fibers. Both observed bundles, one image owner, one final
renderer and one matching action owner are required. Preview, no-auth placeholder,
unknown source/scope, mismatched message, account policy or image are rejected
without fetching, clicking or reporting that the website lacks image support.

Only the selected pointer and validated conversation scope reach the shared
download owner. The native layer receives an opaque handle, not asset pointers,
signed URLs or credentials. Every click requests fresh authorization. Expiring
preview signatures do not invalidate a still-current original; changed account,
document, route, message, pointer or watermark choice does. A late response after
selection changes cannot enqueue the old image.

Versions: generated reader 1; message reader 2; file downloader 37; adapter 363.
No new UI, duplicate file downloader, speculative endpoint or automatic write
replay was added. Original descriptors remain absent from persistent snapshots.

## Verification

- 60 targeted Node tests passed across message images, image gallery and files.
- All 126 production adapter assets parsed successfully.
- Synthetic cases cover ordinary/project scopes, paid/free defaults, watermarked
  originals, missing/duplicate owners, previews, identity changes and late replies.
- Wireless ADB remained connected to the same Xiaomi hardware. No recording,
  private conversation changes, Cookie clearing or proxy changes were performed.
- The initial source batch did not build/install an APK. The later grouped 1684
  release passed 29 Android unit tests, normal build/install and actual native
  original-image download/decode; see the linked acceptance report.

This reader binds originals to committed generated images. The separate
[history projection](chatgpt-generated-image-history-20260912.md) later added
typed generated images, and its owned sample file index/download also passed on
1684. Hidden/tool-message text is not broadly exposed by either path.
Compact multi-image rails, old DALL-E owners, absent inline overlays and shared
conversation variants remain outside this verified source contract.
