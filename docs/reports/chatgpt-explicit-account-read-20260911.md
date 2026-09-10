# Explicit account reads and prefetch isolation

Scope: production ChatGPT conversation file index, project membership probe and
attachment scope read. This is regression evidence, not a new protocol contract.

## Observed on normal 1648

`file-reference-inventory-batch-1648-20260911-062517-206` visited six bounded
fixture candidates through the native navigation owner. All six file-index
receipts returned `files_not_ready`; original conversation/draft and the awake
lease were restored. No messages, file downloads or source writes occurred.
The bounded protocol capture contained successful official responses, including
one conversation detail 200. Several requests were unfinished when capture
stopped and records were dropped; status 0 is not proof of a network failure.

The initial inventory's `passed` meant only that the scan and restoration
finished. It did NOT mean that the file reads passed. The harness now separately
reports `inventory_completed`; `passed` requires a nonempty candidate set, every
index read succeeding, and restoration. No citation/mounted acceptance is
claimed from the old `passed` field.

## Code findings and correction

- Private transport 25 used the background history policy's cooldown to gate
  explicit file reads, membership probes and attachment scope reads. A failed
  or empty prefetch could disable those independent user operations.
- Transport 26 keeps one existing GET/single-flight implementation but separates
  persisted `account_read` health from background prefetch health. Both retain
  bounded timeouts. This is not a second request implementation or DOM loop.
- Authentication rejection and rate limiting remain shared protection. Migration
  retains the old protection's remaining deadline without extending it.
- File reads acquire/join the existing identity promise rather than failing
  from a momentary readiness boolean. `auth_cooldown`/unavailable preparation
  does not become a new five-minute transport failure. Identity acquisition
  still owns its own timeout/cooldown; no login bypass or automatic write replay.
- Receipts distinguish `files_disabled`, `files_read_cooldown`,
  `files_identity_unavailable` and actual `files_read_failed`.

The exact internal health state behind every 1648 live rejection was not
exposed. The coupling above is reproduced in deterministic tests, not inferred
as the sole cause of all live failures.

## Verification

- `account-read-file-regression-fixed-20260911-064036-131`: eight Node test
  files, 108 runner tests, zero failures; includes file indexing, citations,
  mounted download and mounted attachment regressions. One existing bridge
  upgrade test was corrected to compare against the current module version,
  not the obsolete hardcoded library version 9.
- Inventory PowerShell parser and read-only/restore/failure contract: passed.
- `account-read-native-20260911-064127-534`: `BUILD SUCCESSFUL`, 16 Android
  tests across file parsing/cache, MCP and production file-list presentation,
  zero failures/errors. Adapter bundle syntax: 110 assets passed.
- Updated-device acceptance and release: pending this batch.

File citation saved bytes and mounted-source download/attachment still require
their own real workflow evidence. A successful index is not a downloaded file,
and none of those pending capabilities is marked completed by this correction.
