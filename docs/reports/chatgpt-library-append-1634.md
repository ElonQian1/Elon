# Library Append Delivery 1634

## Delivery

2026-09-10, normal Release **1.1.1634 (1634)**, adapter **317**.
Source: `9b3bc765204cabf80009103f14e082ee68dd07f9`.
APK SHA-256: `8b79c4f3a75833c1deb22c9ae02635786ec3669c9a12203a1fe4baf1d52b088c`.

`library-notification-release-20260910-145727-297` completed successfully:
Gradle 6m27s, publication verified, wireless replacement installation succeeded.
The phone's MCP subsequently reported version 1634 and adapter 317. No application
data, Cookies or login were cleared. No proxy or audio implementation changed.

Earlier normal 1632 bundled the Library deadline and append changes, but device
acceptance exposed the completion-notification race described in the
[implementation record](../chatgpt-private-library-append.md). Version 1634 fixes
that race. Both artifacts are full Release builds, not research/debug packages.

## Verification

- The regression on pre-fix code returned `library_attachment_unconfirmed` after
  publishing a ready file. `library-notification-red-20260910-145031-661` fails at
  that exact assertion. The production notification cancels attachment work while
  invalidating the text context; the old receipt race interpreted this as failure.
- After moving notification behind settled completion, the related suite passed
  **192/192**, no skipped/cancelled cases:
  `library-notification-related-20260910-145207-782`. It includes the actual
  cancelling callback behavior for ordinary and mounted references. The earlier
  append implementation's separate 284-case run is not counted as another 284
  unique tests of this fix.
- `library-append-1634-native-20260910-150611-959` clicked the production Library
  entry, searched the fixed TXT fixture, selected it and invoked Attach through
  the rendered Android controls. It observed a successful
  `library_attachment_associated` receipt, one ready private card, a remove control
  and a closed Library browser. This accepts the first-file completion fix.
- That native run stopped during the second selection because the external
  accessibility runner reported `foreground_package_mismatch`. A resume later
  stopped at `semantic_node_missing`. Neither run sent a message. A separate
  window-owner check confirmed MainActivity remained focused; no screenshot or
  coordinate click was substituted. The complete multi-file button flow is not
  accepted from these partial runs.
- `library-append-1634-mcp-20260910-151025-886` then exercised the same production
  attachment dispatch through APK MCP, independently of external control lookup.
  First TXT association succeeded; second PDF returned
  **`library_attachment_policy_unconfirmed`** and preserved the first ready file.
  No send was attempted. This is a real remaining policy-resolution failure, not
  proof that ChatGPT lacks multiple attachments and not a passing append test.

The MCP runner reported an empty composer and restored the awake lease. A later
receipt audit showed its removal request and original-conversation restoration
timed out, so that runner flag alone does **not** establish full restoration.
The first earlier TXT was removed with a confirmed receipt; the later cleanup
needs a fresh ready-page check. A single non-destructive page refresh was requested
after confirming no draft, active audio, message send or remote mutation. No remote
Library records were deleted. Final restoration confirmation remains pending.

## Remaining Work

1. Resolve the actual committed-owner/current-limit shape causing the second-file
   policy result. Preserve official quota checks and existing ready files; do not
   bypass the guard merely to make the test green.
2. Accept sequential references and their single private-runtime send after that
   fix. The prepared two-document prompt was never sent in this batch.
3. Address visible Library entries outliving the catalog's 60-second selection
   TTL. The stale selection was independently observed on 1632.
4. New local-picker files after staged Library references remain outside the
   implemented append contract. Mounted real-device materialization and its
   deadline remain separate pending checks; first-file TXT success does not
   certify them.

The overarching private-interface Goal remains active. No global completion,
thermal improvement, Google acceptance or complete multi-file success is claimed.
