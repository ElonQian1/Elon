# SQLite Safe Snapshot

`@yilong/sqlite-safe-snapshot` creates transaction-consistent SQLite backup artifacts, verifies
their digest and schema, and restores a reviewed artifact into a new database file. It is shared by
merchant projects that use the ERP SQLite adapter or the open-commerce runtime idempotency store.

The package uses Node's SQLite online backup API. It does not copy the main database file, so a
committed transaction still present in WAL is included in the snapshot.

## Runtime

Use Node 22.16 or newer. The current repository acceptance runs on Node 24.8.0, where `node:sqlite`
still emits an experimental warning. Importing the existing ERP or connector root entry does not
load this package.

## Create And Verify

All paths must be absolute. The destination directory must already exist and the destination file
must not exist.

```js
import { resolve } from "node:path";
import {
  createSqliteSnapshot,
  verifySqliteSnapshot,
} from "@yilong/sqlite-safe-snapshot";

const receipt = await createSqliteSnapshot({
  sourcePath: resolve("./data/erp.sqlite3"),
  destinationPath: resolve("./backups/erp-2026-08-16.sqlite3"),
  expectedUserVersion: 1,
  requiredTables: ["yilong_erp_records", "yilong_erp_audit"],
});

await verifySqliteSnapshot({
  path: resolve("./backups/erp-2026-08-16.sqlite3"),
  expectedSha256: receipt.sha256,
  expectedUserVersion: 1,
  requiredTables: ["yilong_erp_records", "yilong_erp_audit"],
});
```

For `@elon/open-commerce-connector/sqlite-idempotency`, use user version `0` and require:

- `yilong_merchant_runtime_idempotency`
- `yilong_merchant_runtime_idempotency_meta`

The receipt contains only operation metadata, SHA-256, byte length, SQLite user version, table
names, copied pages and creation time. It does not contain local paths or business rows.

## Restore To A New File

Stop the runtime that will consume the restored database before switching configuration. Restore
requires the exact SHA-256 reviewed during backup verification and refuses an existing target.

```js
import { resolve } from "node:path";
import { restoreSqliteSnapshot } from "@yilong/sqlite-safe-snapshot";

const restored = await restoreSqliteSnapshot({
  sourcePath: resolve("./backups/erp-2026-08-16.sqlite3"),
  destinationPath: resolve("./data/erp-restored.sqlite3"),
  expectedSha256: "<64 lowercase hex characters>",
  expectedUserVersion: 1,
  requiredTables: ["yilong_erp_records", "yilong_erp_audit"],
});
```

After restore, open the new file with the owning store, run a business read smoke, then change the
runtime configuration under the merchant's normal deployment and rollback process. Keep the old
database until the restored runtime is accepted. The SDK intentionally has no overwrite option.

## Publication And Failure Rules

- Snapshot and restore write a random temporary SQLite file in the destination directory.
- The temporary file must pass `PRAGMA quick_check`, version and required-table validation.
- The SDK streams SHA-256, flushes the file, then publishes it with an atomic hard link.
- Two callers racing for one destination cannot overwrite each other; at most one publishes.
- Direct symbolic-link sources, symbolic-link target parents, corrupt databases, hash drift,
  missing tables and schema mismatch fail closed.
- Failed normal operations remove their private temporary file and never publish the final target.

This is an SDK primitive, not a backup scheduler or a production disaster-recovery service. It does
not upload, encrypt, rotate, retain or restore in place; it does not provide multi-machine HA,
multi-primary replication, network-disk consensus or a completed production recovery drill.
