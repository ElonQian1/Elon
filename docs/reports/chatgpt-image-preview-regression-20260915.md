# Native Conversation Image Preview Regression

## Scope And Status

- Capability: `android_chatgpt_conversation_image_preview_lifecycle_v1`.
- Production surface: ordinary friend chat / ChatGPT, not the old test page.
- Code: implemented; device acceptance and release evidence pending below.
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
  including a 256px source rendered at 16px. Live classification remains to be checked.
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
  policy 3); main and test Kotlin compiled. Canonical APK release pending.
- Device: pending same-conversation native rendering after installation.

## Acceptance Boundary

Do not mark all image scenarios complete from these checks. The immediate check is
the user's current text/search conversation; valid generated-image rendering and
download must remain intact. Record failed, deferred and completed cases separately.
