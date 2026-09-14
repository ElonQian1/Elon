# Tool Ownership And Native Image Admission

Baseline: `e772da2e7`, APK 1731 / adapter 396. This batch addresses remaining
independent Image-send admission; it does not repeat accepted Search sends or
Writing Block functionality.

## Source Correction

The private tool context captured its conversation through `#prompt-textarea`
but independently captured permissions/model through the tool trigger. A missing,
replaced or unrelated editor therefore rejected an otherwise committed tool owner.
Context v5 uses the same committed trigger for both captures. It retains the
existing runtime-profile, identity, route, shared/file-store, controller, model,
temporary-mode and filtered-menu checks. No background polling, guessed capability,
second writer, credential export or request replay was added. Adapter is 397.

`tool-owner-baseline-20260914-201526-102` reproduced the three absent/unrelated
editor cases. `tool-owner-verified-20260914-201742-741` passed 306 tests, zero
failures/skips, covering tool selection, runtime send, native input ownership,
fresh text context and the bounded probe. The shared probe test still expected
version 25 although the unchanged implementation already upgraded to 26; its
stale assertion was corrected, not the runtime behavior.

## Device Evidence Before This Release

- `fresh-image-native-dispatch-20260914-201131-267`: stopped before navigation
  because the APK was not foreground. Zero sends and zero pending writes.
- After opening production native ChatGPT, `fresh-image-production-dispatch-20260914-201229-594`
  stopped in tool preparation on a missing semantic control. The tool context,
  adapter, composer and document continuity were ready; zero sends/POSTs.
- `tool-native-projection-inspect-20260914-201714-074` and
  `tool-close-control-inspect-20260914-201943-193` inspected the owned fixture
  without changing tools or sending. Search was selected; its native chip and
  enabled/visible close child were present. The latter explicitly checked the
  close control rather than inferring it from a chip label. Original navigation
  and awake leases were restored. The earlier transient control miss is not
  evidence of a permanently missing native feature.

The external semantic inspector now reports only bounded tool-control booleans
and dimensions, never message text or credentials.

## Release And Follow-up

`tool-owner-release-20260914-202227-515` built, published and installed 1733
(adapter 397), source `ce03609eb`, SHA-256
`c4b752516e75a44432b7e297a1fcb2170860510f3f8fb379477fe4ec0d4ca21c`.
The post-install production session was authenticated, idle and adapter-current.
`fresh-image-1733-native-20260914-203403-630` then stopped before sending:
the official trigger disappeared during navigation, producing `composer_detached`.
The route and awake lease were restored; no pending write was created.

Context v6 reuses the existing bounded committed-composer locator when the
trigger is absent. The runtime-profile owner must be unique, committed to the
current root, and share the current conversation/controller/stores. Restoring
the same trigger does not invalidate an otherwise unchanged tool binding.
This is a page-local official state command, not a new HTTP tool-selection API;
it does not manufacture eligibility or bypass authentication. General native
text-owner selection retains its prior behavior and limits.

`tool-memory-owner-final-20260914-204044-626`: 322 tests passed, zero skipped.
New tests cover an absent editor and trigger, same-owner DOM remount, duplicate
profiled owners, and identity/route/model/profile/root/controller/permission drift.
No writer or DOM fallback runs when an owned selection becomes stale.
Versions: committed-owner locator 2, text-runtime capture 22, tool context 6,
private tool selector 4, adapter 398.

## Remaining

Context v6 shipped with Writing Block parser 8 in APK 1734 / adapter 399,
source `724392dcb`. `fresh-image-1734-native-20260914-211441-484` passed tool
context and native Image selection but stopped before sending. Added diagnostic
stages preserve the original control step and script filename across cleanup.
`fresh-image-1734-stages-20260914-211837-656` then identified a missing native
`clear_search` control despite the private catalog confirming selected Search.
Neither attempt sent a request or left a pending write; route/awake state restored.

`tool-projection-readonly-20260914-212654-122` reproduced the projection gap:
the catalog changed from empty to selected Search, but the chip/close control
remained absent after both list and dismiss receipts. No tool mutation or send.
The session only notified the native model-menu callback for ComposerControls;
tool changes waited for a later message snapshot, which may be deduplicated.
The callback now carries the section and immediately refreshes the native
composer for all sections, while only model options enter the model popup.
No polling, reload, protocol gate relaxation or extra write was introduced.

`composer-state-notification-tests-20260914-213200-597`: 100 tests passed,
zero skipped. The source contract also verifies canonical-state commit before
notification, all-section delivery, model behavior and selected-state retention.
Release/device verification of the notification fix and one actual independent
Image send remain pending. No Image default/completed marker is enabled from
menu admission alone. Preserve the fixture ledger and reconcile uncertain writes
read-only; do not repeat the accepted Search send.
