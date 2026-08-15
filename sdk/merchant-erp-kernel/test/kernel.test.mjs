import assert from "node:assert/strict";
import test from "node:test";

import {
  ErpKernelError,
  MemoryErpStore,
  createMerchantErpKernel,
} from "../src/index.js";

function fixture() {
  const storage = new MemoryErpStore({
    stores: [
      { id: "store_a", merchant_id: "merchant_a", name: "Store A", status: "active" },
      { id: "store_b", merchant_id: "merchant_a", name: "Store B", status: "active" },
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
        id: "tea",
        merchant_id: "merchant_a",
        store_id: "store_a",
        sku: "TEA-1",
        name: "Tea",
        category: "drink",
        currency: "CNY",
        unit_price_micro: 1_800_000,
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
        quantity: 10,
        revision: 1,
      },
      {
        merchant_id: "merchant_a",
        store_id: "store_a",
        product_id: "tea",
        quantity: 1,
        revision: 1,
      },
    ],
  });
  let sequence = 0;
  const kernel = createMerchantErpKernel({
    merchantId: "merchant_a",
    store: storage,
    clock: () => 1_700_000_000_000,
    idFactory: () => `id${++sequence}`,
    plugins: [
      {
        key: "coffee_labels",
        enrichCatalogItem: (item) => (item.product_id === "coffee" ? { badge: "popular" } : null),
      },
    ],
  });
  return { kernel, storage };
}

test("isolates merchants and keeps UI concerns outside the kernel", async () => {
  const { kernel } = fixture();
  assert.deepEqual(await kernel.listStores(), {
    stores: [
      { id: "store_a", name: "Store A", status: "active" },
      { id: "store_b", name: "Store B", status: "active" },
    ],
  });
  const catalog = await kernel.searchCatalog({ store_id: "store_a", query: "coffee" });
  assert.equal(catalog.items.length, 1);
  assert.equal(catalog.items[0].unit_price_micro, 2_500_000);
  assert.deepEqual(catalog.items[0].extensions.coffee_labels, { badge: "popular" });
  assert.equal("theme" in kernel, false);
});

test("records purchase, inventory and balanced journal atomically", async () => {
  const { kernel, storage } = fixture();
  const request = {
    store_id: "store_a",
    idempotency_key: "purchase_001",
    currency: "CNY",
    supplier_ref: "supplier-local",
    items: [{ product_id: "coffee", quantity: 5, unit_cost_micro: 1_000_000 }],
  };
  const first = await kernel.recordPurchase(request);
  const replay = await kernel.recordPurchase(request);
  assert.deepEqual(replay, first);
  const snapshot = await storage.snapshot();
  assert.equal(snapshot.purchases.length, 1);
  assert.equal(snapshot.journals.length, 1);
  assert.equal(snapshot.inventory.find((item) => item.product_id === "coffee").quantity, 15);
  assert.deepEqual(
    snapshot.journals[0].lines.map((line) => [line.direction, line.amount_micro]),
    [
      ["debit", 5_000_000],
      ["credit", 5_000_000],
    ],
  );
  await assert.rejects(
    kernel.recordPurchase({ ...request, supplier_ref: "changed" }),
    (error) => error instanceof ErpKernelError && error.code === "IDEMPOTENCY_CONFLICT",
  );
});

test("creates one unpaid order and never double-decrements inventory", async () => {
  const { kernel, storage } = fixture();
  const request = {
    store_id: "store_a",
    idempotency_key: "order_001",
    items: [{ product_id: "coffee", quantity: 2 }],
  };
  const first = await kernel.createOrder(request);
  const replay = await kernel.createOrder(request);
  assert.deepEqual(replay, first);
  assert.equal(first.payment_status, "unpaid");
  assert.equal(first.total_micro, 5_000_000);
  const status = await kernel.getOrder({ store_id: "store_a", order_id: first.order_id });
  assert.deepEqual(status, first);
  const snapshot = await storage.snapshot();
  assert.equal(snapshot.orders.length, 1);
  assert.equal(snapshot.inventory.find((item) => item.product_id === "coffee").quantity, 8);
});

test("rolls back all order changes when one line has insufficient inventory", async () => {
  const { kernel, storage } = fixture();
  await assert.rejects(
    kernel.createOrder({
      store_id: "store_a",
      idempotency_key: "order_002",
      items: [
        { product_id: "coffee", quantity: 2 },
        { product_id: "tea", quantity: 2 },
      ],
    }),
    (error) => error instanceof ErpKernelError && error.code === "INSUFFICIENT_INVENTORY",
  );
  const snapshot = await storage.snapshot();
  assert.equal(snapshot.orders.length, 0);
  assert.equal(snapshot.inventory.find((item) => item.product_id === "coffee").quantity, 10);
  assert.equal(snapshot.inventory.find((item) => item.product_id === "tea").quantity, 1);
});
