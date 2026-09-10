# Native Library Attachment Append

## Scope And Status

The production Library Attach action can append to an already-owned native
attachment collection: consecutive Library files and Library files after a
staged private local-upload batch. It reuses the existing composer owner,
removal path, native cards and official-runtime submit lease. Concrete mounted
files reuse the existing private materialization request; ordinary Library
files retain their backing IDs without download/reupload.

Code is implemented and offline verified. Adapter 316, attachment composer 21,
sender 23, Library attachment 4 and runtime bindings 9 are prepared for the
next grouped APK. Published 1631 and currently installed 1630 do not contain this
change. Production rendered multi-file attachment/send acceptance is pending.
This is not an independent Android HTTP text sender.

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

Real native UI attach/send and provider-policy shapes remain to be verified in
the grouped phone round. Adding new local picker files *after* staged Library
references remains a separate empty-composer upload limitation; this change does
not silently broaden that contract. Foreign/unowned or uploading file stores,
project/temporary Library references, live/suggestion cloud references, folder
writes and new provider shapes are not covered. No evidence-backed folder-write
endpoint was found in the inspected Library sources; none was invented.
