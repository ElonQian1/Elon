# Fresh Retry Runtime Parent, September 14

## Device Evidence

On production APK 1721, `fresh-retry-native-1721-current-20260914-111224-923`
created a controlled conversation, completed its initial native send, retained a
synthetic unsent draft and clicked native regenerate once. Fresh HTTP rejected
before dispatch with `scope_unsupported`; its state had no accepted request and
zero stream events. The existing runtime compatibility path then returned
`official_runtime_v1:regenerate_unknown:timeout_stream_missing`.

This was **not successful independent retry acceptance**. The later snapshot
still reported streaming and retained the draft. No second retry, navigation,
reload, installation, draft clearing or Cookie reset was performed. A subsequent
read-only protocol-state probe also timed out. Do not replay this uncertain
runtime write; reconcile it read-only before another device test.

## Reproduced Source Defect

The pinned `web_20260912` shared bundle has `Cx.getParent()` read a node's
`parentId`; `getNodeIfExists()` returns that runtime node without conversion.
Private history mappings use `parent` instead. Fresh retry incorrectly used the
wire field on the runtime node. Its synthetic fixture made the same mistake,
concealing the refusal. This explains a reproducible pre-dispatch scope rejection,
not every possible runtime timeout.

Context v2 (adapter 390) now reads a strict string `parentId` for capture and
ownership rechecks. The fixture explicitly converts runtime `parentId` to wire
`parent` and back during reconciliation. Tests reject malformed or wire-shaped
runtime nodes and invalidate ownership if the parent changes. No scope, model,
draft, account or uncertain-write guard was relaxed. Pinned SHA/AST evidence checks
the actual public implementation without executing downloaded provider code.

The retry harness also reuses the reviewed version 7/8 idle-state validator and
preserves an explicitly requested current native surface instead of re-launching
the app during MCP bootstrap. Its UI route and draft checks remain in place.

## Validation And Delivery Boundary

- Red: `fresh-retry-runtime-parent-red-20260914-112628-591` reproduced
  `scope_unsupported` using the corrected runtime-shaped fixture.
- Green: `fresh-retry-runtime-parent-regression-20260914-112920-224` passed
  **294 Node tests**, zero failures/cancellations/skips, in 12.9 seconds. This
  includes fresh send/retry/stop/history and pinned public-source evidence.
- This fix is source-only pending the next grouped APK build. Installed/published
  1721 does not include adapter 390. No new Android compilation or corrected
  independent retry device acceptance is claimed here.

The independent retry capability remains opt-in and not production-verified.
Writing-block editing/export and accepted personal first/follow-up sending keep
their existing completed markers; this diagnostic does not reopen those scopes.
