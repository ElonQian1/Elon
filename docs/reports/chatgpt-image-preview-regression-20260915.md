# Native Conversation Image Preview Regression

## Scope And Status

- Capability: `android_chatgpt_conversation_image_preview_lifecycle_v1`.
- Production surface: ordinary friend chat / ChatGPT, not the old test page.
- Code: implemented and published in normal 1.1.1770; reported-answer native
  structural acceptance passed. A full visual/all-image acceptance is not claimed.
- This does not reopen completed image-gallery creation or original-download work.
  Those samples did not establish that every inline conversation preview worked.

## Evidence Before Changes

- User reported persistent "preparing image preview" placeholders in a text/search answer.
- Wireless ADB reconnected to the approved Xiaomi; installed version 1.1.1767,
  adapter 428, native and backing conversation each had five messages; voice idle.
- MCP reported image part counts 5 / 13 / 2 across the three assistant messages.
  Existing MCP metadata did not expose image dimensions or preview registration.
- Normal-release WebView debugging was disabled. The temporary CDP forward was
  removed; no research flags, credentials, private content, or page state changed.
- Synthetic source-icon regression fails against base `2d43562fd`: only linked
  images had a small-icon filter. Unlinked sources-button favicons became image parts,
  including a 256px source rendered at 16px. See the bounded live result below.
- Native mapping treated any missing local image with a handle as pending, even
  after the coordinator recorded failure. Repeated snapshots could also requeue
  the active handle, leaving an extra attempt after retry exhaustion.

## Changes

- Adapter 429 excludes small rendered icons before image registration regardless
  of link wrapping. Large/unloaded previews and narrow diagrams remain image parts.
- Per-handle preview state reaches native bubbles: idle, preparing, failed, cached.
  Failures offer click-to-retry rather than permanent preparing text.
- Repeated snapshots/taps cannot duplicate the active request; exhausted handles
  are not pumped again without retry. Stale unowned events and post-reset disk
  callbacks do not restart the queue.
- MCP image metadata adds only dimensions and a registration boolean, no URLs,
  handles, credentials, or image bytes.
- No cookie/data clearing, new private endpoint, proxy change, or voice change.

## Verification

- Node run `image-preview-node-20260916-20260915-234217-079`: passed five targeted
  suites covering extraction, image assets, lifecycle, private originals and content.
- Baseline source-icon assertion reproduced separately against `2d43562fd`.
- Android run `image-preview-android-release-20260915-20260915-234446-622`:
  21 focused release unit tests passed (mapper 11, store 1, lifecycle 6, rich-content
  policy 3); main and test Kotlin compiled.
- Normal release run `image-preview-release-20260915-20260915-235441-560` passed
  build, signing, upload, remote SHA/size and version checks in 486.9 seconds.
  Source `a4ad3a9bb`, adapter 429, version 1.1.1770 (1770), 40,733,186 bytes;
  SHA-256 `ab156d0ff4c233043aa9c9140cbad696d86c0470fc4ef548326f91ca3a80c3f4`.
- ADB replacement installation returned Success; package manager confirmed 1770.
  Autoinstall was disabled only for this publish invocation. Installation followed
  an explicit MCP idle/non-streaming/non-dictating readback after waking the app.
- Production `social_ai`, adapter 429, bridge ready, authenticated, no login required.
  Original five-message identity digest still matched; two user-originated later
  messages were present, with seven messages in both native and backing models.
- The reported answer retained 689 characters and its image count changed 2 -> 0;
  `chatgpt_reveal_message` succeeded on that exact original answer. The earlier
  answer retained 5,702 characters and changed 13 -> 3 images, including registered
  1920x1005 / 980x653 media. Thus this is not an indiscriminate image-removal fix.
- The oldest cached answer still had five image descriptors without registered
  previews. That historical reconstruction path and successful rendering of the
  three remaining media previews were not accepted in this batch.
- Native hierarchy capture could not reach idle within its bounded timeout.
  Temporary device XML was removed; no private hierarchy or screenshot was saved.
  Semantic/model checks above are device evidence, not a claim of screenshot QA.
- No new messages, recordings, downloads of private originals, or account changes
  were initiated. Cookie/application data and existing conversations were retained.

## Acceptance Boundary

Do not mark all image scenarios complete from these checks. The immediate check is
the user's current text/search conversation; valid generated-image rendering and
download must remain intact. Record failed, deferred and completed cases separately.
