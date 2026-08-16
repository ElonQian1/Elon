import assert from "node:assert/strict";
import { createHmac } from "node:crypto";
import test from "node:test";

import {
  createMemoryMerchantRuntimeIdempotencyStore,
  createMerchantRuntime,
} from "../../open-commerce-connector/src/index.js";
import {
  createMerchantErpKernel,
  createMerchantRuntimeBinding,
  createOpenCommerceProvider,
  MemoryErpStore,
} from "../src/index.js";

const merchantId = "merchant_runtime_erp";
const keyId = "OPEN_COMMERCE_RUNTIME_ERP_TEST";
const secret = "merchant-erp-runtime-secret-that-is-at-least-32-bytes";
const nowUnix = 1_786_345_200;

test("binds the generic ERP kernel to the signed merchant runtime", async () => {
  const store = createStore();
  const kernel = createKernel(store);
  const binding = createMerchantRuntimeBinding(createOpenCommerceProvider(kernel));
  const createCapability = binding.capabilities.find((item) => item.key === "order.create");

  assert.equal(binding.schema, "yilong.erp.merchant_runtime_binding.v1");
  assert.equal(binding.merchantId, merchantId);
  assert.equal(createCapability.action, true);
  assert.equal(createCapability.input_schema.properties.idempotency_key, undefined);
  assert.equal(createCapability.input_schema.required.includes("idempotency_key"), false);
  assert.equal(
    binding.capabilities.find((item) => item.key === "catalog.search").action,
    false,
  );

  const runtime = createRuntime(binding);
  const orderInput = {
    store_id: "main",
    items: [{ product_id: "coffee", quantity: 1 }],
  };
  const missingConfirmation = await runtime.handleInvoke(signedInvocation({
    capability_key: "order.create",
    grant_id: "grant_erp_1",
    idempotency_key: "platform_order_1",
    input: orderInput,
  }));

  assert.equal(missingConfirmation.status, 400);
  assert.equal(missingConfirmation.body.error_code, "confirmation_required");
  assert.equal((await store.snapshot()).orders.length, 0);

  const first = await runtime.handleInvoke(signedInvocation({
    capability_key: "order.create",
    grant_id: "grant_erp_1",
    action_confirmation_id: "confirmation_erp_1",
    idempotency_key: "platform_order_1",
    input: orderInput,
  }));

  assert.equal(first.status, 200);
  assert.equal(first.body.result.payment_status, "unpaid");
  assert.deepEqual(first.body.result._yilong_business_receipt, {
    schema: "open_commerce.merchant_business_receipt.v1",
    entity_type: "order",
    reference_id: first.body.result.order_id,
    state: "awaiting_payment",
    occurred_at: "2026-08-16T00:00:00.000Z",
  });

  const status = await runtime.handleInvoke(signedInvocation({
    invocation_id: "invocation_erp_status",
    capability_key: "order.status",
    grant_id: "grant_erp_1",
    idempotency_key: "platform_status_1",
    input: { store_id: "main", order_id: first.body.result.order_id },
  }));
  assert.equal(status.status, 200);
  assert.equal(status.body.result.order_id, first.body.result.order_id);
  assert.deepEqual(
    status.body.result._yilong_business_receipt,
    first.body.result._yilong_business_receipt,
  );

  const restarted = createRuntime(binding);
  const replay = await restarted.handleInvoke(signedInvocation({
    invocation_id: "invocation_erp_2",
    capability_key: "order.create",
    grant_id: "grant_erp_1",
    action_confirmation_id: "confirmation_erp_2",
    idempotency_key: "platform_order_1",
    input: orderInput,
  }));
  const snapshot = await store.snapshot();

  assert.equal(replay.status, 200);
  assert.equal(replay.body.result.order_id, first.body.result.order_id);
  assert.equal(snapshot.orders.length, 1);
  assert.equal(
    snapshot.inventory.find((item) => item.product_id === "coffee").quantity,
    4,
  );
})

test("fails closed on ERP idempotency conflicts and merchant identity mismatch", async () => {
  const store = createStore();
  const binding = createMerchantRuntimeBinding(
    createOpenCommerceProvider(createKernel(store)),
  );
  const runtime = createRuntime(binding);
  const base = {
    capability_key: "order.create",
    grant_id: "grant_erp_2",
    action_confirmation_id: "confirmation_erp_3",
    idempotency_key: "platform_order_conflict",
    input: {
      store_id: "main",
      items: [{ product_id: "coffee", quantity: 1 }],
    },
  };
  assert.equal((await runtime.handleInvoke(signedInvocation(base))).status, 200);

  const conflictingRuntime = createRuntime(binding);
  const conflict = await conflictingRuntime.handleInvoke(signedInvocation({
    ...base,
    invocation_id: "invocation_erp_conflict",
    input: {
      store_id: "main",
      items: [{ product_id: "coffee", quantity: 2 }],
    },
  }));
  assert.equal(conflict.status, 500);
  assert.equal(conflict.body.error_code, "internal_error");

  const mismatchedRuntime = createRuntime(binding, "merchant_other");
  const mismatch = await mismatchedRuntime.handleInvoke(signedInvocation({
    ...base,
    invocation_id: "invocation_erp_mismatch",
    merchant_id: "merchant_other",
    idempotency_key: "platform_order_mismatch",
  }));
  const snapshot = await store.snapshot();

  assert.equal(mismatch.status, 500);
  assert.equal(mismatch.body.error_code, "internal_error");
  assert.equal(snapshot.orders.length, 1);
  assert.equal(
    snapshot.inventory.find((item) => item.product_id === "coffee").quantity,
    4,
  );
})

function createStore() {
  return new MemoryErpStore({
    stores: [{ id: "main", merchant_id: merchantId, name: "Main" }],
    products: [{
      id: "coffee",
      merchant_id: merchantId,
      store_id: "main",
      sku: "coffee-1",
      name: "Coffee",
      currency: "CNY",
      unit_price_micro: 2_500_000,
    }],
    inventory: [{
      merchant_id: merchantId,
      store_id: "main",
      product_id: "coffee",
      quantity: 5,
      revision: 0,
    }],
  });
}

function createKernel(store) {
  let sequence = 0;
  return createMerchantErpKernel({
    merchantId,
    store,
    clock: () => "2026-08-16T00:00:00.000Z",
    idFactory: () => `erp_${++sequence}`,
  });
}

function createRuntime(binding, runtimeMerchantId = binding.merchantId) {
  return createMerchantRuntime({
    merchantId: runtimeMerchantId,
    keyId,
    secret,
    capabilities: binding.capabilities,
    handlers: binding.handlers,
    idempotencyStore: createMemoryMerchantRuntimeIdempotencyStore(),
  });
}

function signedInvocation(overrides = {}) {
  const envelope = {
    schema: "merchant_runtime.invoke.v1",
    invocation_id: "invocation_erp_1",
    merchant_id: merchantId,
    capability_key: "catalog.search",
    requester_user_id: "consumer_erp_1",
    requester_app_id: "app_erp_1",
    credential_environment: "sandbox",
    credential_id: "credential_erp_1",
    grant_id: null,
    action_confirmation_id: null,
    idempotency_key: "platform_query_1",
    issued_at_unix: nowUnix,
    input: { store_id: "main" },
    ...overrides,
  };
  const body = JSON.stringify(envelope);
  const signature = createHmac("sha256", secret)
    .update(String(nowUnix), "utf8")
    .update(".")
    .update(body)
    .digest("hex");
  return {
    body,
    headers: {
      "x-yilong-runtime-key-id": keyId,
      "x-yilong-runtime-timestamp": String(nowUnix),
      "x-yilong-runtime-signature": `v1=${signature}`,
    },
    nowUnix,
  };
}
