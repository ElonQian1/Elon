# Independent Tool Send Device Evidence

Date: 2026-09-14. Device candidate: APK 1728, adapter 395.

## Search

`fresh-search-native-owned-20260914-190651-188` passed:

- One production native Send action and one successful send receipt.
- Independent fresh page HTTP owner; no seed send or fallback replay.
- 32 stream events; completed source-linked citation output in native chat.
- Terminal history reconciliation confirmed. Observation interval: 15,412 ms,
  including UI/MCP polling, not a first-token measurement.
- Original route and screen-awake setting restored; private content not exported.

The original report did not independently confirm asynchronous tool deselection.
A later read found Search selected again on re-entry. Native cancellation was
then confirmed by a new successful receipt and a fresh unselected catalog in
`fresh-search-owned-tool-cleanup-20260914-192219-556` (19.8 seconds, zero sends).
Re-entering still restored Search selection; this is observed state inheritance,
not proof of a failed private Search request or a diagnosed website defect.

The acceptance script now waits for cancellation receipt and catalog agreement
before navigation. It may clear an inherited tool only in the exact synthetic
fixture with an accepted owner ledger; unknown/user-owned selections remain
untouched. This is a per-page cleanup guarantee, not persistent tool-preference
storage across navigation.

## Image Candidate

No Image request has been sent by these attempts:

- The inherited Search selection initially blocked preflight.
- `fresh-image-owned-inherited-selection-20260914-192540-281` cleared it and
  selected Image, then rejected ambiguous matching of an old test bubble and
  the collapsed composer. The temporary Image selection and draft were restored.
- The semantic runner now matches the unique per-attempt synthetic prefix for
  both composer opening and Send. It does not click an arbitrary matching bubble.
- `fresh-image-unique-native-draft-20260914-192846-378` stopped earlier because
  private tool ownership was `conversation_unavailable`, despite cached composer
  readiness. No trial/send was started and the original route was restored.

Image remains unverified under the independent sender. Do not classify a host
binding/preflight failure as an HTTP endpoint rejection, and do not repeatedly
resend the already accepted Search turn to unlock Image testing.

## Validation And Delivery

39 synthetic evidence tests and 33 source/semantic wiring checks passed. These
cover stale/failed/ambiguous receipts, unknown selection, changed draft/route,
and ordering cancellation confirmation before restoring the original view.

151 JavaScript tests passed in
`fresh-search-final-scope-guards-20260914-193349-451` (1.8 seconds, zero skips).
Only [existing-personal Search](../chatgpt-fresh-search-text-dispatch.md) gains
default admission. The Image, new-tool, project, temporary and attachment scopes
are not promoted.

## Released Default

`fresh-search-default-release-20260914-193852-096` passed in 468.6 seconds.
APK `1.1.1731 / 1731`, source `e97e6fffe`, adapter 396, 40,479,563 bytes;
SHA-256 `0f81fcd08f2ae3a4801aed92033f9375eaae5226c6b645bf8d9cc49cda27b6aa`.
The release entrypoint published it and verified an unattended Xiaomi update.
No Cookies, login state or application data were cleared.

Both changed JavaScript assets match their source hashes inside the release APK.
Post-install production MCP confirmed adapter 396/current, authenticated, composer
ready, not streaming and voice idle. This check sent zero messages; it is not a
second Search sample or an Image pass. The native runner also compiled and ran
its read-only inspection; the unique Image draft click remains untested because
the final candidate preflight stopped before that step.

[Writing Blocks](../chatgpt-writing-blocks-native.md) are reused unchanged:
native read/edit/export and ordinary cloud save are accepted; special cloud-save
variants still need their own owned samples. This batch does not mark those
variants completed or rebuild their editor.
