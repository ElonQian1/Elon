import assert from "node:assert/strict";
import test from "node:test";

import {
  MemoryErpStore,
  createMerchantErpKernel,
  createOpenCommerceProvider,
} from "../src/index.js";

test("maps the same merchant kernel to consumer AI capabilities", async () => {
  const storage = new MemoryErpStore({
    stores: [{ id: "main", merchant_id: "merchant", name: "Main" }],
    products: [
      {
        id: "sku_1",
        merchant_id: "merchant",
        store_id: "main",
        sku: "SKU-1",
        name: "Product",
        currency: "CNY",
        unit_price_micro: 1_000_000,
      },
    ],
    inventory: [
      {
        merchant_id: "merchant",
        store_id: "main",
        product_id: "sku_1",
        quantity: 3,
        revision: 1,
      },
    ],
  });
  const kernel = createMerchantErpKernel({
    merchantId: "merchant",
    store: storage,
    idFactory: () => "stable",
    clock: () => 1_700_000_000_000,
  });
  const provider = createOpenCommerceProvider(kernel);
  assert.equal(provider.schema, "yilong.erp.open_commerce_provider.v1");
  assert.deepEqual(
    provider.capabilities.map((capability) => capability.capability_key),
    ["catalog.search", "inventory.query", "order.create", "order.status"],
  );
  assert.equal(
    provider.capabilities.find((capability) => capability.capability_key === "order.create").action,
    true,
  );
  const order = await provider.invoke({
    capability_key: "order.create",
    input: {
      store_id: "main",
      idempotency_key: "consumer_order_1",
      items: [{ product_id: "sku_1", quantity: 1 }],
    },
  });
  assert.equal(order.payment_status, "unpaid");
  const status = await provider.invoke({
    capability_key: "order.status",
    input: { store_id: "main", order_id: order.order_id },
  });
  assert.deepEqual(status, order);
});

test("does not expose capabilities whose modules are disabled", () => {
  const kernel = createMerchantErpKernel({
    merchantId: "merchant",
    store: new MemoryErpStore(),
    enabledModules: ["catalog"],
  });
  const provider = createOpenCommerceProvider(kernel);
  assert.deepEqual(provider.capabilities.map((item) => item.capability_key), ["catalog.search"]);
});
