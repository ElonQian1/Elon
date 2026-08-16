import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { DatabaseSync } from "node:sqlite";
import test from "node:test";

import {
  ErpKernelError,
  createMerchantErpKernel,
} from "../src/index.js";
import {
  SQLITE_ERP_SCHEMA_VERSION,
  SqliteErpStore,
} from "@yilong/merchant-erp-kernel/sqlite";

function seed(quantity = 10) {
  return {
    stores: [
      { id: "store_a", merchant_id: "merchant_a", name: "Store A", status: "active" },
      { id: "store_a", merchant_id: "merchant_b", name: "Other", status: "active" },
    ],
    products: [
      {
        id: "coffee",
        merchant_id: "merchant_a",
        store_id: "store_a",
        sku: "COFFEE-1",
        name: "Coffee",
        category: "drink",
        currency: "CNY",
        unit_price_micro: 2_500_000,
        active: true,
      },
      {
        id: "coffee",
        merchant_id: "merchant_b",
        store_id: "store_a",
        sku: "OTHER-1",
        name: "Other Coffee",
        currency: "CNY",
        unit_price_micro: 9_900_000,
        active: true,
      },
    ],
    inventory: [
      {
        merchant_id: "merchant_a",
        store_id: "store_a",
        product_id: "coffee",
        quantity,
        revision: 1,
      },
    ],
  };
}

function kernelFor(storage, start = 0, plugins = []) {
  let sequence = start;
  return createMerchantErpKernel({
    merchantId: "merchant_a",
    store: storage,
    clock: () => 1_700_000_000_000,
    idFactory: () => `sqlite${++sequence}`,
    plugins,
  });
}

async function temporaryDatabase() {
  const directory = await mkdtemp(join(tmpdir(), "yilong-erp-sqlite-"));
  return { directory, path: join(directory, "merchant.sqlite3") };
}

test("persists purchases, orders, idempotency and audit across reopen", async (t) => {
  const temporary = await temporaryDatabase();
  let firstStore;
  let reopenedStore;
  t.after(async () => {
    await Promise.allSettled([firstStore?.close(), reopenedStore?.close()]);
    await rm(temporary.directory, { recursive: true, force: true });
  });
  firstStore = new SqliteErpStore({ path: temporary.path, seed: seed() });
  const firstKernel = kernelFor(firstStore);
  const purchaseInput = {
    store_id: "store_a",
    idempotency_key: "purchase_001",
    currency: "CNY",
    supplier_ref: "supplier-local",
    items: [{ product_id: "coffee", quantity: 5, unit_cost_micro: 1_000_000 }],
  };
  const purchase = await firstKernel.recordPurchase(purchaseInput);
  assert.deepEqual(await firstKernel.recordPurchase(purchaseInput), purchase);
  const orderInput = {
    store_id: "store_a",
    idempotency_key: "order_001",
    items: [{ product_id: "coffee", quantity: 2 }],
  };
  const order = await firstKernel.createOrder(orderInput);
  assert.deepEqual(await firstKernel.createOrder(orderInput), order);
  await firstStore.close();

  reopenedStore = new SqliteErpStore({ path: temporary.path });
  const reopenedKernel = kernelFor(reopenedStore, 100);
  assert.equal(reopenedStore.schemaVersion, SQLITE_ERP_SCHEMA_VERSION);
  assert.deepEqual(
    await reopenedKernel.getOrder({ store_id: "store_a", order_id: order.order_id }),
    order,
  );
  const snapshot = await reopenedStore.snapshot();
  assert.equal(snapshot.purchases.length, 1);
  assert.equal(snapshot.journals.length, 1);
  assert.equal(snapshot.orders.length, 1);
  assert.equal(snapshot.idempotency.length, 2);
  assert.equal(snapshot.audit.length, 2);
  assert.equal(snapshot.inventory.find((item) => item.product_id === "coffee").quantity, 13);
  assert.equal(snapshot.stores.some((item) => item.merchant_id === "merchant_b"), true);
});

test("rolls back every record when a storage transaction fails", async (t) => {
  const store = new SqliteErpStore({ path: ":memory:", seed: seed() });
  t.after(() => store.close());

  await assert.rejects(
    store.transact((transaction) => {
      transaction.putInventory({
        merchant_id: "merchant_a",
        store_id: "store_a",
        product_id: "coffee",
        quantity: 99,
        revision: 2,
      });
      transaction.insertOrder({
        id: "partial_order",
        merchant_id: "merchant_a",
        store_id: "store_a",
      });
      throw new Error("fixture transaction failure");
    }),
    /fixture transaction failure/,
  );

  const snapshot = await store.snapshot();
  assert.equal(snapshot.orders.length, 0);
  assert.equal(snapshot.inventory[0].quantity, 10);
  await assert.rejects(
    store.read((transaction) => transaction.putInventory(snapshot.inventory[0])),
    (error) => error instanceof ErpKernelError && error.code === "READ_ONLY_TRANSACTION",
  );
});

test("serializes one adapter and never double-decrements inventory", async (t) => {
  const store = new SqliteErpStore({ path: ":memory:", seed: seed(3) });
  t.after(() => store.close());
  const kernel = kernelFor(store);
  const [first, second] = await Promise.allSettled([
    kernel.createOrder({
      store_id: "store_a",
      idempotency_key: "order_a",
      items: [{ product_id: "coffee", quantity: 2 }],
    }),
    kernel.createOrder({
      store_id: "store_a",
      idempotency_key: "order_b",
      items: [{ product_id: "coffee", quantity: 2 }],
    }),
  ]);
  assert.equal(first.status, "fulfilled");
  assert.equal(second.status, "rejected");
  assert.equal(second.reason.code, "INSUFFICIENT_INVENTORY");
  const snapshot = await store.snapshot();
  assert.equal(snapshot.orders.length, 1);
  assert.equal(snapshot.inventory[0].quantity, 1);
});

test("fails closed with a structured error when another connection owns the writer lock", async (t) => {
  const temporary = await temporaryDatabase();
  const firstStore = new SqliteErpStore({
    path: temporary.path,
    seed: seed(),
    busyTimeoutMs: 25,
  });
  const secondStore = new SqliteErpStore({ path: temporary.path, busyTimeoutMs: 25 });
  t.after(async () => {
    await Promise.allSettled([firstStore.close(), secondStore.close()]);
    await rm(temporary.directory, { recursive: true, force: true });
  });

  let release;
  let markStarted;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  const started = new Promise((resolve) => {
    markStarted = resolve;
  });
  const first = firstStore.transact(async (transaction) => {
    const stock = transaction.getInventory("merchant_a", "store_a", "coffee");
    transaction.putInventory({ ...stock, quantity: 9, revision: stock.revision + 1 });
    markStarted();
    await gate;
  });
  await started;
  await assert.rejects(
    secondStore.transact((transaction) => {
      const stock = transaction.getInventory("merchant_a", "store_a", "coffee");
      transaction.putInventory({ ...stock, quantity: 8, revision: stock.revision + 1 });
    }),
    (error) => error instanceof ErpKernelError && error.code === "STORAGE_BUSY",
  );
  release();
  await first;

  const snapshot = await secondStore.snapshot();
  assert.equal(snapshot.inventory[0].quantity, 9);
  assert.equal(snapshot.inventory[0].revision, 2);
});

test("rejects reseeding business data and unknown future schema versions", async (t) => {
  const temporary = await temporaryDatabase();
  const futureDirectory = await mkdtemp(join(tmpdir(), "yilong-erp-future-"));
  const corruptDirectory = await mkdtemp(join(tmpdir(), "yilong-erp-corrupt-"));
  t.after(async () => {
    await Promise.all([
      rm(temporary.directory, { recursive: true, force: true }),
      rm(futureDirectory, { recursive: true, force: true }),
      rm(corruptDirectory, { recursive: true, force: true }),
    ]);
  });
  const seeded = new SqliteErpStore({ path: temporary.path, seed: seed() });
  await seeded.close();
  assert.throws(
    () => new SqliteErpStore({ path: temporary.path, seed: seed() }),
    (error) => error instanceof ErpKernelError && error.code === "SEED_REQUIRES_EMPTY_DATABASE",
  );

  const futurePath = join(futureDirectory, "future.sqlite3");
  const future = new DatabaseSync(futurePath);
  future.exec(`PRAGMA user_version = ${SQLITE_ERP_SCHEMA_VERSION + 1}`);
  future.close();
  assert.throws(
    () => new SqliteErpStore({ path: futurePath }),
    (error) => error instanceof ErpKernelError && error.code === "UNSUPPORTED_SCHEMA_VERSION",
  );

  const corruptPath = join(corruptDirectory, "corrupt.sqlite3");
  const corrupt = new DatabaseSync(corruptPath);
  corrupt.exec(`
    CREATE TABLE yilong_erp_records (kind TEXT);
    CREATE TABLE yilong_erp_audit (sequence INTEGER);
    PRAGMA user_version = ${SQLITE_ERP_SCHEMA_VERSION};
  `);
  corrupt.close();
  assert.throws(
    () => new SqliteErpStore({ path: corruptPath }),
    (error) => error instanceof ErpKernelError && error.code === "STORAGE_CORRUPTED",
  );
});
