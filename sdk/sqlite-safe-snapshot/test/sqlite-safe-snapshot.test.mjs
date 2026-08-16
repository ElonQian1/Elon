import assert from "node:assert/strict";
import { mkdir, mkdtemp, readdir, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";
import test from "node:test";

import {
  createSqliteSnapshot,
  restoreSqliteSnapshot,
  SQLITE_SAFE_SNAPSHOT_RECEIPT_SCHEMA,
  verifySqliteSnapshot,
} from "../src/index.js";
import { createMerchantErpKernel } from "../../merchant-erp-kernel/src/index.js";
import { SqliteErpStore } from "../../merchant-erp-kernel/src/sqlite-store.js";
import { createSqliteMerchantRuntimeIdempotencyStore } from "../../open-commerce-connector/src/merchant-runtime-sqlite-store.js";

const ERP_TABLES = Object.freeze(["yilong_erp_audit", "yilong_erp_records"]);
const IDEMPOTENCY_TABLES = Object.freeze([
  "yilong_merchant_runtime_idempotency",
  "yilong_merchant_runtime_idempotency_meta",
]);

async function temporaryDirectory(t) {
  const directory = await mkdtemp(join(tmpdir(), "yilong-sqlite-snapshot-"));
  const resources = [];
  t.after(async () => {
    for (const resource of resources.reverse()) await resource.close();
    await rm(directory, { recursive: true, force: true });
  });
  return {
    directory,
    track(resource) {
      resources.push(resource);
      return resource;
    },
  };
}

function assertCode(code) {
  return (error) => {
    assert.equal(error?.code, code);
    return true;
  };
}

async function assertNoTemporaryFiles(directory) {
  const names = await readdir(directory);
  assert.equal(names.some((name) => name.endsWith(".sqlite-snapshot-tmp")), false);
}

test("creates, verifies and restores an online WAL snapshot", async (t) => {
  const { directory, track } = await temporaryDirectory(t);
  const sourcePath = join(directory, "live.sqlite");
  const snapshotPath = join(directory, "live.snapshot.sqlite");
  const restoredPath = join(directory, "live.restored.sqlite");
  const database = track(new DatabaseSync(sourcePath));
  database.exec(`
    PRAGMA journal_mode = WAL;
    PRAGMA user_version = 7;
    CREATE TABLE events (id INTEGER PRIMARY KEY, value TEXT NOT NULL) STRICT;
    INSERT INTO events (value) VALUES ('committed-before-snapshot');
  `);

  const snapshot = await createSqliteSnapshot({
    sourcePath,
    destinationPath: snapshotPath,
    expectedUserVersion: 7,
    requiredTables: ["events"],
  });
  assert.equal(snapshot.schema, SQLITE_SAFE_SNAPSHOT_RECEIPT_SCHEMA);
  assert.equal(snapshot.operation, "snapshot");
  assert.match(snapshot.sha256, /^[a-f0-9]{64}$/);
  assert.ok(snapshot.size_bytes > 0);
  assert.ok(snapshot.pages_copied > 0);
  assert.deepEqual(snapshot.tables, ["events"]);

  database.exec("INSERT INTO events (value) VALUES ('after-snapshot')");
  const verified = await verifySqliteSnapshot({
    path: snapshotPath,
    expectedSha256: snapshot.sha256,
    expectedUserVersion: 7,
    requiredTables: ["events"],
  });
  assert.equal(verified.sha256, snapshot.sha256);

  const restored = await restoreSqliteSnapshot({
    sourcePath: snapshotPath,
    destinationPath: restoredPath,
    expectedSha256: snapshot.sha256,
    expectedUserVersion: 7,
    requiredTables: ["events"],
  });
  assert.equal(restored.operation, "restore");
  assert.equal(restored.source_sha256, snapshot.sha256);
  const restoredDatabase = track(new DatabaseSync(restoredPath, { readOnly: true }));
  assert.deepEqual(
    restoredDatabase.prepare("SELECT value FROM events ORDER BY id").all().map((row) => row.value),
    ["committed-before-snapshot"],
  );
  await assertNoTemporaryFiles(directory);
});

test("backs up and restores the merchant ERP database without losing records", async (t) => {
  const { directory, track } = await temporaryDirectory(t);
  const sourcePath = join(directory, "erp.sqlite");
  const snapshotPath = join(directory, "erp.snapshot.sqlite");
  const restoredPath = join(directory, "erp.restored.sqlite");
  let sequence = 0;
  const store = track(new SqliteErpStore({
    path: sourcePath,
    seed: {
      stores: [{ id: "store_a", merchant_id: "merchant_a", name: "Store A" }],
      products: [
        {
          id: "coffee",
          merchant_id: "merchant_a",
          store_id: "store_a",
          sku: "COFFEE-1",
          name: "Coffee",
          currency: "CNY",
          unit_price_micro: 2_500_000,
          active: true,
        },
      ],
      inventory: [
        {
          merchant_id: "merchant_a",
          store_id: "store_a",
          product_id: "coffee",
          quantity: 10,
          revision: 1,
        },
      ],
    },
  }));
  const kernel = createMerchantErpKernel({
    merchantId: "merchant_a",
    store,
    clock: () => 1_700_000_000_000,
    idFactory: () => `snapshot${++sequence}`,
  });
  await kernel.recordPurchase({
    store_id: "store_a",
    idempotency_key: "purchase_snapshot",
    currency: "CNY",
    supplier_ref: "supplier-local",
    items: [{ product_id: "coffee", quantity: 5, unit_cost_micro: 1_000_000 }],
  });
  const order = await kernel.createOrder({
    store_id: "store_a",
    idempotency_key: "order_snapshot",
    items: [{ product_id: "coffee", quantity: 2 }],
  });
  const expected = await store.snapshot();

  const snapshot = await createSqliteSnapshot({
    sourcePath,
    destinationPath: snapshotPath,
    expectedUserVersion: 1,
    requiredTables: ERP_TABLES,
  });
  await restoreSqliteSnapshot({
    sourcePath: snapshotPath,
    destinationPath: restoredPath,
    expectedSha256: snapshot.sha256,
    expectedUserVersion: 1,
    requiredTables: ERP_TABLES,
  });

  const restoredStore = track(new SqliteErpStore({ path: restoredPath }));
  assert.deepEqual(await restoredStore.snapshot(), expected);
  const restoredKernel = createMerchantErpKernel({
    merchantId: "merchant_a",
    store: restoredStore,
  });
  assert.deepEqual(
    await restoredKernel.getOrder({ store_id: "store_a", order_id: order.order_id }),
    order,
  );
  await assertNoTemporaryFiles(directory);
});

test("restores a completed merchant runtime idempotency result", async (t) => {
  const { directory, track } = await temporaryDirectory(t);
  const sourcePath = join(directory, "idempotency.sqlite");
  const snapshotPath = join(directory, "idempotency.snapshot.sqlite");
  const restoredPath = join(directory, "idempotency.restored.sqlite");
  const input = {
    merchantId: "merchant_a",
    requesterAppId: "app_a",
    capabilityKey: "order.create",
    idempotencyKey: "stable-order-key",
    invocationId: "invocation_a",
    requestHash: "a".repeat(64),
  };
  const store = track(createSqliteMerchantRuntimeIdempotencyStore({ path: sourcePath }));
  assert.deepEqual(await store.claim(input), { status: "claimed" });
  assert.equal(await store.complete(input, { order_id: "order_a", status: "unpaid" }), true);

  const snapshot = await createSqliteSnapshot({
    sourcePath,
    destinationPath: snapshotPath,
    expectedUserVersion: 0,
    requiredTables: IDEMPOTENCY_TABLES,
  });
  await restoreSqliteSnapshot({
    sourcePath: snapshotPath,
    destinationPath: restoredPath,
    expectedSha256: snapshot.sha256,
    expectedUserVersion: 0,
    requiredTables: IDEMPOTENCY_TABLES,
  });
  const restoredStore = track(
    createSqliteMerchantRuntimeIdempotencyStore({ path: restoredPath }),
  );
  assert.deepEqual(await restoredStore.claim(input), {
    status: "replayed",
    result: { order_id: "order_a", status: "unpaid" },
  });
  await assertNoTemporaryFiles(directory);
});

test("fails closed for target, digest, schema and corrupt artifact errors", async (t) => {
  const { directory } = await temporaryDirectory(t);
  const sourcePath = join(directory, "source.sqlite");
  const existingPath = join(directory, "existing.sqlite");
  const database = new DatabaseSync(sourcePath);
  database.exec("PRAGMA user_version = 3; CREATE TABLE expected (id INTEGER PRIMARY KEY) STRICT");
  database.close();
  await writeFile(existingPath, "reserved");

  await assert.rejects(
    createSqliteSnapshot({ sourcePath, destinationPath: existingPath }),
    assertCode("SNAPSHOT_TARGET_EXISTS"),
  );
  await assert.rejects(
    createSqliteSnapshot({ sourcePath, destinationPath: sourcePath }),
    assertCode("SNAPSHOT_PATH_CONFLICT"),
  );
  await assert.rejects(
    verifySqliteSnapshot({ path: sourcePath, expectedSha256: "0".repeat(64) }),
    assertCode("SNAPSHOT_HASH_MISMATCH"),
  );
  await assert.rejects(
    verifySqliteSnapshot({ path: sourcePath, expectedUserVersion: 4 }),
    assertCode("SNAPSHOT_SCHEMA_MISMATCH"),
  );
  await assert.rejects(
    verifySqliteSnapshot({ path: sourcePath, requiredTables: ["missing"] }),
    assertCode("SNAPSHOT_SCHEMA_MISMATCH"),
  );
  const corruptPath = join(directory, "corrupt.sqlite");
  await writeFile(corruptPath, "not a sqlite database");
  await assert.rejects(
    verifySqliteSnapshot({ path: corruptPath }),
    assertCode("SNAPSHOT_SQLITE_INVALID"),
  );
  await assert.rejects(
    restoreSqliteSnapshot({ sourcePath, destinationPath: join(directory, "new.sqlite") }),
    /expectedSha256 is required/,
  );
  await assertNoTemporaryFiles(directory);
});

test("publishes at most one snapshot when two operations race", async (t) => {
  const { directory } = await temporaryDirectory(t);
  const sourcePath = join(directory, "source.sqlite");
  const targetPath = join(directory, "winner.sqlite");
  const database = new DatabaseSync(sourcePath);
  database.exec("CREATE TABLE records (id INTEGER PRIMARY KEY) STRICT; INSERT INTO records DEFAULT VALUES");
  database.close();

  const results = await Promise.allSettled([
    createSqliteSnapshot({ sourcePath, destinationPath: targetPath, requiredTables: ["records"] }),
    createSqliteSnapshot({ sourcePath, destinationPath: targetPath, requiredTables: ["records"] }),
  ]);
  assert.equal(results.filter((result) => result.status === "fulfilled").length, 1);
  const rejected = results.find((result) => result.status === "rejected");
  assert.equal(rejected.reason.code, "SNAPSHOT_TARGET_EXISTS");
  await verifySqliteSnapshot({ path: targetPath, requiredTables: ["records"] });
  await assertNoTemporaryFiles(directory);
});

test("rejects a direct symbolic-link source and target parent", async (t) => {
  const { directory } = await temporaryDirectory(t);
  const sourceDirectory = join(directory, "source-directory");
  const sourceLinkPath = join(directory, "source-link");
  await mkdir(sourceDirectory);
  await symlink(
    sourceDirectory,
    sourceLinkPath,
    process.platform === "win32" ? "junction" : "dir",
  );
  await assert.rejects(
    verifySqliteSnapshot({ path: sourceLinkPath }),
    assertCode("SNAPSHOT_SYMBOLIC_LINK_REJECTED"),
  );

  const databasePath = join(directory, "source.sqlite");
  const database = new DatabaseSync(databasePath);
  database.exec("CREATE TABLE records (id INTEGER PRIMARY KEY) STRICT");
  database.close();
  const targetDirectory = join(directory, "target-directory");
  const targetLinkPath = join(directory, "target-link");
  await mkdir(targetDirectory);
  await symlink(
    targetDirectory,
    targetLinkPath,
    process.platform === "win32" ? "junction" : "dir",
  );
  await assert.rejects(
    createSqliteSnapshot({
      sourcePath: databasePath,
      destinationPath: join(targetLinkPath, "snapshot.sqlite"),
    }),
    assertCode("SNAPSHOT_SYMBOLIC_LINK_REJECTED"),
  );
});
