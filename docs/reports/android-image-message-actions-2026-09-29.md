# Android Image Message Actions

Scope: restore normal message actions for image attachments in APK chat. No server,
Windows transport, login, proxy, or private-API changes.

## Cause and Fix

- `SourceLinkViews.wrapImage` installed a QR-only long-click listener. The message
  listener binder preserved that specialized listener, hiding the normal menu.
  Interactive chat images now use the message menu; QR recognition remains an
  option there. Read-only image views retain their QR menu.
- Copy, forward, and AI-reply eligibility relied on nonempty message text.
  `ChatMessageContentActions` includes attachment-only messages and rejects recalled
  content. Existing quote, favorite, multi-select, and AI-analysis routes are reused.
- Recycled attachment views retained old generic listeners. Owned listeners now
  refresh or clear on rebinding without overwriting voice-specific listeners.
- Copy and forward prepare original attachment bytes off the main thread, through
  a narrow FileProvider cache, using read-only content-URI grants. System forwarding
  retains captions and is compatible with the existing image share receiver.
  No chooser recipient is selected and no group message is sent automatically.
- Limits: six attachments, 12 MiB each; bounded local reads and remote cache reads;
  canonical local paths restricted to app-owned attachment caches; original
  image-format validation; 192 MiB export-cache cap; only export files older
  than 24 hours expire. An unavailable attachment fails the whole share instead of
  silently substituting a filename. Account changes, recalls, and attachment edits
  during preparation prevent clipboard/chooser delivery.

## Verification

- Targeted Robolectric tests: 28 passed, zero failures, in
  `image-message-tests-verified-20260929-163039-486`.
- Fixture corrections: native bitmap encoding/decoding; a Windows-only FileProvider
  path adapter (production XML root still checked); Java IO access opened only for
  test JVMs so Robolectric can close ParcelFileDescriptor handles. No production
  validation was weakened to make fixtures pass.
- Source-size guard, document-modularity guard, and mobile V2 governance checks
  passed. Governance checks do not constitute visual acceptance.
- Release build passed (`image-message-apk-20260929-163441-732`, Gradle 9m10s).
- APK 1.1.1836 installed on the registered Xiaomi with data-preserving replacement;
  registered Honor offline, verification deferred. Remote APK SHA-256 and size checked.
- Device semantic acceptance: `image_quote` checks copy, forward, quote, multi-select,
  favorite, AI reply, and QR menu entries, then quotes and cancels without changing
  the existing draft or sending any message. Passed on the updated Xiaomi APK.
- Post-install app launch returned to the conversation list; `open_fixture` restored
  the authorized test group before `image_quote`. No existing messages were sent,
  edited, or deleted. No screenshot or coordinate input was needed.
- Tests cover original-byte preservation, file URI/MIME grants, multi-file payloads,
  image import through the existing share receiver, invalid/oversize files, bounded
  cache expiry, listener recycling, selection exit, and full-size image tapping.
- Full external-app paste/send compatibility is not claimed. Receiving applications
  must support Android image clipboard or share intents.

## Delivery

- Code commit: `90100fb23` (rebased after a real non-fast-forward rejection).
  Concurrent PWA media changes were preserved; they did not change APK sources.
- Published APK: 1.1.1836, source `90100fb23`.
- SHA-256: `491e3854dbcfdee0d241650c1b20737646f3027c0779d3bec06eeadce82bcab3`.
- The Windows checkout is retained clean after finalization to preserve the verified
  build and avoid the preceding task's Windows residual-directory cleanup failure.
