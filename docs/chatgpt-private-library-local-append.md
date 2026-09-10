# Local Upload After Library Selection

## Status

Capability: `android_chatgpt_private_library_local_upload_append_v1`.
Code: implemented. Verification: offline_verified. Completion: not yet accepted
on a device. Delivery: grouped Android build pending, adapter 321, composer 22,
sender 26, attachment policy 3 and runtime bindings 10.

This is the reverse-order extension of [Library append](chatgpt-private-library-append.md),
not a new uploader. Ordinary authenticated Library selection followed by one
or several native local files now reuses the current private upload transport,
native attachment cards and complete official-runtime submit lease. The accepted
1635 consecutive-Library case remains completed and does not need new research.

Project/temporary Library references remain outside this capability. Their
already-accepted empty-composer local uploads are unchanged. Additional local
quota bindings are evidenced only for the September 10 website profile; older
profiles keep their previously supported behavior, without guessed exports.

## Ownership And Publication

The former sender admitted only `composer.available()`, which requires an empty
FilePicker store. The new nonempty path captures the existing private collection
through `captureUpload()`. It does not relax that empty-store predicate or adopt
foreign/unowned files. Library and upload capture share the same complete ready
collection verification and exact submit lease.

The captured account, document token, route, model, store, file objects, metadata
and ordering remain current during authentication, native byte reads, image
preparation and upload. Removing an old card, adding a foreign item, changing
model/context or cancelling invalidates the pending operation. Each batch is
published atomically after all files are processed, preserving old native IDs
and File identities. Partial failure never replaces the previous selection or
replays a write through another path. Readback rollback removes only newly added
items, preserving both old and concurrently added foreign entries.

A reused backing ID already selected with the same Library ID is a no-op. A
contradictory processed result with that ID is rejected. The next submit lease
contains the whole updated collection; an earlier unsubmitted lease becomes
stale. This module does not send chat text itself.

## Official Quota Evidence

Public assets were read without account credentials. Exact files under
`https://chatgpt.com/cdn/assets/`:

| Asset | SHA-256 |
|---|---|
| `8b34dbc2-l0q54hwyus1gmzd1.js` | `22842341683a190c8b9c197e103a2d537faa88fc8d54b110efaeb8b199c9df57` |
| `conversation-small-ng27r04netsz3e4o.js` | `00ae8fd8e7a9639909d773aca4aa79d86e8834a49307aaa3794a23690b5885dd` |
| `4813494d-ilaxclpwvg5i0e40.js` | `3f28b7c766311f6a6a2ebf6986e872571459d694b58468fef5a68fe13f1787a4` |

The composer's `uploadFile` checks both total pending files against the current
type-specific remaining allowance and counted pending uploads against the
configured per-turn limit. Its Library-reference validator alone is not enough:
image and document upload base limits differ. Current menu quota props may
legitimately be explicitly undefined when the quota banner is not enabled.

The existing versioned resolver adds these **export aliases**, distinct from
minified local symbols or similarly named exports used by other consumers:

| Normalized binding | Official export | Public local symbol |
|---|---|---|
| shared `attachmentUploadType` | `Up` | `dX` |
| conversation `attachmentBaseLimit` | `eQt` | `C0t` |
| conversation `attachmentMaxUploads` | `sQt` | `F0t` |
| conversation `attachmentPendingCount` | `nQt` | `b0t` |
| conversation `attachmentConfiguredLimit` | `$Zt` | `L0t` |

Policy 3 reuses the committed composer/menu owner and its exact FilePicker
context. It requires current file-upload eligibility and calls these live
official helpers plus the existing size validator. It checks the whole proposed
batch before byte reads, again during upload and before publication, retaining
the native nine-file cap. Count-only prospective entries never enter the official
store. Missing bindings or invalid limits remain unconfirmed, not evidence that
the website lacks attachments. A confirmed quota rejection gets a limit message,
not a connection-error message.

## Verification And Next Acceptance

The new integration fixture exercises the real sender, composer, Library
association and quota policy with synthetic byte/transport boundaries. Red run:
`library-local-append-red-20260910-173533-400`. Final related run:
`library-local-append-final-20260910-174300-615`: **311 passed**, zero failures,
skips or cancellations across 16 targeted suites/files. Production asset assembly
syntax is included; this is not an Android build or live protocol acceptance.

Cases cover Library then single/batch/repeated local selection, image-specific
and changing quotas, absent quota props, unknown runtime, cancellation,
account/document/route/model changes, removal, foreign additions, late completion,
partial processing, duplicate reuse, contradictory receipts, atomic readback
rollback and complete submit-lease invalidation. Existing project, temporary,
reservation and runtime-submit contracts also passed.

One grouped production-native acceptance must select a synthetic Library fixture,
add a different local fixture through the production picker, confirm both cards,
send once and verify both fixture contents. Restore the original conversation,
draft and attachment state. Then record the actual APK/source/receipt evidence;
do not reuse 1635's forward-order acceptance as proof of this reverse order.
