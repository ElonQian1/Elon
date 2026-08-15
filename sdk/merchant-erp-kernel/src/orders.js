import { fail } from "./errors.js";
import {
  appendAudit,
  replayOrReject,
  requireModule,
  requireProduct,
  requireStore,
  saveIdempotency,
} from "./runtime.js";
import {
  checkedAdd,
  checkedMultiply,
  optionalText,
  requireId,
  requirePositiveInteger,
  requireUniqueItems,
} from "./validation.js";

function normalizeCreate(input) {
  requireUniqueItems(input.items, "product_id", "items");
  return {
    store_id: requireId(input.store_id, "store_id"),
    idempotency_key: requireId(input.idempotency_key, "idempotency_key"),
    customer_ref: optionalText(input.customer_ref, "customer_ref"),
    items: input.items.map((item) => ({
      product_id: requireId(item.product_id, "items.product_id"),
      quantity: requirePositiveInteger(item.quantity, "items.quantity", 100_000),
    })),
  };
}

export async function createOrder(runtime, rawInput = {}) {
  requireModule(runtime, "order");
  requireModule(runtime, "catalog");
  requireModule(runtime, "inventory");
  const input = normalizeCreate(rawInput);
  return runtime.store.transact(async (tx) => {
    requireStore(tx, runtime.merchantId, input.store_id);
    const identity = {
      merchantId: runtime.merchantId,
      storeId: input.store_id,
      operation: "order.create",
      key: input.idempotency_key,
    };
    const replay = replayOrReject(tx, identity, input);
    if (replay.response) {
      return replay.response;
    }
    for (const plugin of runtime.plugins) {
      if (plugin.validateOrder) {
        await plugin.validateOrder(structuredClone(input));
      }
    }

    let currency = null;
    let totalMicro = 0;
    const lines = [];
    for (const item of input.items) {
      const product = requireProduct(
        tx,
        runtime.merchantId,
        input.store_id,
        item.product_id,
      );
      if (currency && currency !== product.currency) {
        fail("CURRENCY_MISMATCH", "one order cannot contain multiple currencies", 409);
      }
      currency = product.currency;
      const stock = tx.getInventory(runtime.merchantId, input.store_id, item.product_id);
      if (stock.quantity < item.quantity) {
        fail("INSUFFICIENT_INVENTORY", `insufficient inventory for ${item.product_id}`, 409);
      }
      const lineTotal = checkedMultiply(
        product.unit_price_micro,
        item.quantity,
        "order line total",
      );
      totalMicro = checkedAdd(totalMicro, lineTotal, "order total");
      lines.push({
        product_id: product.id,
        sku: product.sku,
        name: product.name,
        quantity: item.quantity,
        unit_price_micro: product.unit_price_micro,
        line_total_micro: lineTotal,
        previous_stock: stock,
      });
    }

    for (const line of lines) {
      tx.putInventory({
        ...line.previous_stock,
        quantity: line.previous_stock.quantity - line.quantity,
        revision: line.previous_stock.revision + 1,
      });
    }
    const createdAt = runtime.now();
    const orderId = runtime.createId("order");
    const order = {
      id: orderId,
      merchant_id: runtime.merchantId,
      store_id: input.store_id,
      customer_ref: input.customer_ref,
      status: "awaiting_payment",
      payment_status: "unpaid",
      currency,
      total_micro: totalMicro,
      items: lines.map(({ previous_stock: _stock, ...line }) => line),
      created_at: createdAt,
      updated_at: createdAt,
    };
    tx.insertOrder(order);
    const response = publicOrder(order);
    appendAudit(tx, runtime, input.store_id, "order.created", orderId, {
      currency,
      total_micro: totalMicro,
      line_count: lines.length,
      payment_status: "unpaid",
    });
    saveIdempotency(tx, identity, replay.requestHash, response, createdAt);
    return response;
  });
}

export async function getOrder(runtime, input = {}) {
  requireModule(runtime, "order");
  const storeId = requireId(input.store_id, "store_id");
  const orderId = requireId(input.order_id, "order_id");
  return runtime.store.read((tx) => {
    requireStore(tx, runtime.merchantId, storeId);
    const order = tx.getOrder(runtime.merchantId, storeId, orderId);
    if (!order) {
      fail("ORDER_NOT_FOUND", "order was not found", 404);
    }
    return publicOrder(order);
  });
}

function publicOrder(order) {
  return {
    order_id: order.id,
    store_id: order.store_id,
    status: order.status,
    payment_status: order.payment_status,
    currency: order.currency,
    total_micro: order.total_micro,
    items: order.items,
    created_at: order.created_at,
    updated_at: order.updated_at,
  };
}
