# File Read Ownership Checkpoint

## Device Check

Normal APK 1.1.1683, adapter 361, was reached over wireless ADB after the USB
connection was reported available. The production native ChatGPT surface was
authenticated. No reinstallation, data clearing, proxy changes, microphone use
or message sends were needed for this checkpoint.

One existing image-generation fixture was identified by an exact synthetic
prompt guard and a single user turn. Its native context had three messages and
three image parts. The previous conversation was restored after inspection.

The first private file-index attempt failed with `files_read_network`. A single
explicit retry succeeded with `private_files_ready` in 735 ms, while the bridge
still reported `connecting`. The official document was complete and the route
matched. This is evidence that this private read does not require composer
readiness; it is not evidence of a reproducible connectivity root cause.

That successful index was empty. It does not identify the generated raster's
original file. No raster was renamed, deleted, downloaded or re-created here.
The known uploaded PNG fixture has artifact type `none`; deleting it would not
prove the unverified `other` raster-artifact deletion branch.

## Reproduced Code Defect

The source owner correctly rejects an old history/source response after an
account, document or route change. However, `context_sources_stale` fell into
the generic network category. That displayed a network-disconnected message
and recorded a connection failure/cooldown for the next explicit read.

Four race cases reproduced this against the previous implementation, including
a route change after the base file snapshot and before source enrichment.
This is a separately reproduced defect, not a claim that it caused the first
device failure above; the installed version did not preserve that distinction.

Private transport version 30 now returns `files_context_changed` and does not
record a health failure for the superseded owner. The native file sheet renders
a context-changed retry message and retains cached rows. Old content remains
rejected. Fresh explicit reads can proceed immediately. Genuine network,
timeout, authentication and rate-limit cooldowns remain unchanged. There is
no automatic read retry, navigation, write replay or permission relaxation.

## Verification And Delivery

The five targeted Node scripts pass: context-source integration and policy,
conversation files, private transport and transport health. Tests check the
old-result rejection, one completion receipt, immediate fresh read, no false
health failure, no extra source request, and the existing real-error guards.
Release main/test compilation and all six native presentation JUnit tests pass,
including the new safe status text and continued visibility of cached file rows.
The logged Gradle run took 331.2 seconds; no APK was assembled or published.

This is a source batch for the next grouped Android release. APK 1683 remains
installed; its successful read is not acceptance of the new race fix. Keep the
broader Goal active and do not repeat previously accepted transfers or voice.

## Original Image UI Gap

The image gallery already has an accepted original-download callback and opaque
private selection. `ChatGptSocialImageContentController.open` does not pass that
callback to `ChatImageViewer` for conversation images. Its local image source
is a bounded, JPEG-encoded preview, not an authorized original download.

Reuse the existing original-download owner when a selected conversation image
can be bound to its private original descriptor. Do not guess a file ID from a
preview, copy the preview as an original, or declare the empty file index proof
that the website lacks the image. That binding and raster-artifact deletion
remain unfinished; this checkpoint neither adds another downloader nor widens
the completed gallery-download scope.
