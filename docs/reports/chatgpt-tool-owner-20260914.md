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

## Remaining

Release/device verification of context v5 and one actual independent Image send
are pending. No Image default or completed marker is enabled from unit tests,
menu admission, UI inspection or a pre-dispatch failure. Preserve the existing
fixture ledger and reconcile uncertain writes read-only; do not repeat Search.
