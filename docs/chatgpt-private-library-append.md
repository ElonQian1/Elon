# Native Library Attachment Append

## Scope And Status

The production Library Attach action can append to an already-owned native
attachment collection: consecutive Library files and Library files after a
staged private local-upload batch. It reuses the existing composer owner,
removal path, native cards and official-runtime submit lease. Concrete mounted
files reuse the existing private materialization request; ordinary Library
files retain their backing IDs without download/reupload.

Normal Release 1632 (adapter 316) was built, published and installed over wireless
ADB with login preserved. Production attachment acceptance then exposed a
completion-notification race, reproduced below. Adapter 317, sender 24 and Library
attachment 5 fix that race; composer 21 and runtime bindings 9 are unchanged.
Normal 1634 accepted the first-file receipt/card/return but rejected a second
PDF. Normal **1635 / adapter 318** now passes consecutive TXT/PDF selection from
the production native Library, two ready cards, one official-runtime send,
both document markers in the reply and cleanup/restoration. This scope is
completed; see [1635 acceptance](reports/chatgpt-library-append-1635.md).
The [1634 report](reports/chatgpt-library-append-1634.md) remains historical
failure evidence. This is not an independent Android HTTP text sender.

Policy 2 / adapter 318 additionally preserve the official composer's explicitly
undefined quota props. This source-backed correction and the new complete flow
have passed, without claiming proof of the exact 1634 failed guard. The read-only
`library_attachment_policy` MCP probe reports a closed reason code from the last
policy attempt without loading modules, invoking the validator or exposing props.

## Failure And Fix

Previously `available()` required zero files and `capture()` used that gate.
Every second Library selection therefore failed before its file reference could
be added. The new `captureLibrary()` uses the existing exact submit lease when
the entire ready collection belongs to this private owner. Local upload's
empty-composer admission is unchanged.

Publication preserves existing File objects, ready metadata and ordering, then
checks the complete store readback. If publication is not confirmed, only new
entries are removed; previous and concurrently added foreign files survive.
Changed document, account, route, model, metadata, removal or cancellation
invalidates the pending append. A new submit lease includes the whole updated
collection and makes an older unsubmitted lease stale. Accepted-send cleanup
still removes only that send's exact files.

Duplicate ordinary backing/Library IDs and mounted source IDs are no-ops,
including when at capacity. A cloud item's stale copied `file_id` is not a
duplicate proof for its current source. Old request receipts never append again.
Unknown materialization outcomes retain the existing no-replay rule.

### Completion Notification Race

On installed 1632, the first controlled TXT reference appeared as a ready private
attachment while the command reported `library_attachment_unconfirmed`. The
production `attachmentChanged` callback invalidates the text context, which calls
the attachment sender's `cancel()`. Library publication previously invoked that
callback while its abort race was still pending. The callback therefore won the
race with a false failure even though the owned file had already been committed.

The completion receipt now settles and releases its abort listener before the
native snapshot callback runs. The existing context invalidation remains intact;
pre-commit cancellation still prevents publication. A regression executes that
cancelling callback after both ordinary selections and mounted materialization.
It fails on 1632's code (`library-notification-red-20260910-145031-661`) and passes
after the change. The related run passes 192 cases with no skips/cancellations
(`library-notification-related-20260910-145207-782`).

The earlier `library_selection_expired` observation followed more than 60 seconds
between directory lookup and attach, matching the existing catalog TTL. It is
separate from the completion race; retrying immediately after a completed refresh
isolated the race. [Selection refresh](chatgpt-private-library-selection-refresh.md)
now implements bounded per-file revalidation. Normal 1636 was installed; adapter
320 additionally fixes structured-metadata comparison and scope-error reporting.
Selection-refresh device acceptance remains unconfirmed, not completed.

## Official Evidence

Inspected public composer assets under `https://chatgpt.com/cdn/assets/`:

| Build | Asset | SHA-256 | Attachment actions export |
|---|---|---|---|
| September 7 | `8b34dbc2-nhot65scqrg20d6p.js` | `36644eb82aac9c399bce384c18140f8c878dd780c8f787440b80f27971729733` | `Oh` |
| September 9 | `8b34dbc2-mx35vjavisrk7hwp.js` | `d2fdfeeb3b225bfa8453f62e208076d3af4005c67cc5133fc91599e989f1a708` | `kh` |
| September 9b | `8b34dbc2-cj4kfo18e1ldvw16.js` | `c24245fc260253db68ae531c586174a3eeff95e694b7c8a0e76eb77eae5fe906` | `kh` |
| September 10 | `8b34dbc2-l0q54hwyus1gmzd1.js` | `22842341683a190c8b9c197e103a2d537faa88fc8d54b110efaeb8b199c9df57` | `Ph` |

The retained September 6 composer exports the same object as `fh`, which is
the stable consumer alias. See the existing [runtime binding evidence](chatgpt-private-runtime-bindings.md)
for the original profile. The new aliases extend that same versioned loader,
not a second importer or broad minified-name search at runtime.

`attachLibraryFile` appends and deduplicates backing/mounted IDs. The September
10 `Evn` owner receives `maxLibraryAttachmentCount` and
`maxTotalLibraryAttachmentCount` and invokes `validateChatAttachment` with the
current picker store, byte size, limits and selected files. The new focused
policy reads those committed-owner props and calls the same validator, without
opening a menu. It rechecks the limits immediately before publication and after
mounted-file preparation. Unknown/mismatched state is unconfirmed, not proof the
website lacks Library support. The existing first-file path is unchanged.

The APK retains its existing nine-item safety ceiling; **nine is not claimed as
the website quota**. Official limits can reject earlier. This batch does not
reimplement quota arithmetic, bypass count/size rules or create a new token copy.
Limits apply to the new append path; other upload contexts retain their existing
admission and are not newly certified by this work.

### Optional Quota Contract

The same September 10 asset constructs both menu props as
`Ts == null ? undefined : computedLimit` (near byte 2894556). Its validator
delegates count checks only when the supplied upload limit is non-null (near
byte 517282). Policy 1 incorrectly demanded numeric values for both props.
Policy 2 requires the properties to exist on the confirmed owner, accepts either
undefined or a nonnegative safe integer, and forwards their exact values to the
official validator. Missing properties, null, strings and invalid numbers still
fail closed. It does not synthesize unlimited values, skip the validator or
remove the APK ceiling. An unavailable runtime, mismatched owner/store/scope or
invalid limit remains an unconfirmed policy, not an absent website feature.

The explicit-undefined regression fails on the previous code
(`library-policy-optional-red-20260910-153536-214`). After the fix, 174 related
tests pass without skips (`library-policy-related-20260910-154046-591`), covering
append ownership, official validation, cancellation, runtime aliases and the
page/native diagnostic vocabulary. The fresh 1635 native flow passes; no detailed
1634 policy receipt exists to identify its exact failed boundary retrospectively.

## Verification

- Baseline `64ebe57bd`: the sequential fixture fails at its second selection
  (`library-append-baseline-proof-20260910-141124-979`). Two earlier baseline
  harness invocations failed in command/module setup and are not product evidence.
- The final related run passed 284 Node cases, no skips/cancellations
  (`library-append-final-checks-20260910-141350-395`), including receipt replay
  and distinct-request single-flight coverage. All 110 asset scripts and their
  assembled adapter passed JavaScript syntax checks. Source-size and commit
  ownership guards passed. Full Android build and device results are separate.
- Tests exercise the real composer/sender/policy with synthetic committed owners:
  sequential and mixed references, cloud materialization, duplicate receipts,
  owner mutation, cancellation, official rejection, policy drift, readback
  rollback, removal and exact submit consumption. Runtime aliases are separately
  checked against all five evidenced profiles.

## Remaining Boundaries

Ordinary consecutive TXT/PDF append/send is accepted on 1635 and should be reused.
Adding new local picker files *after* staged Library
references remains a separate empty-composer upload limitation; this change does
not silently broaden that contract. Foreign/unowned or uploading file stores,
project/temporary Library references, live/suggestion cloud references, folder
writes and new provider shapes are not covered. No evidence-backed folder-write
endpoint was found in the inspected Library sources; none was invented.
