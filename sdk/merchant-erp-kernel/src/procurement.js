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
  requireCurrency,
  requireId,
  requireNonNegativeInteger,
  requirePositiveInteger,
  requireUniqueItems,
} from "./validation.js";
import { fail } from "./errors.js";

function normalize(input) {
  const storeId = requireId(input.store_id, "store_id");
  const idempotencyKey = requireId(input.idempotency_key, "idempotency_key");
  const currency = requireCurrency(input.currency);
  const creditAccount = input.credit_account ?? "accounts_payable";
  if (!["accounts_payable", "cash"].includes(creditAccount)) {
    fail("INVALID_INPUT", "credit_account must be accounts_payable or cash");
  }
  requireUniqueItems(input.items, "product_id", "items");
  return {
    store_id: storeId,
    idempotency_key: idempotencyKey,
    currency,
    supplier_ref: optionalText(input.supplier_ref, "supplier_ref"),
    credit_account: creditAccount,
    items: input.items.map((item) => ({
      product_id: requireId(item.product_id, "items.product_id"),
      quantity: requirePositiveInteger(item.quantity, "items.quantity"),
      unit_cost_micro: requireNonNegativeInteger(
        item.unit_cost_micro,
        "items.unit_cost_micro",
      ),
    })),
  };
}

export async function recordPurchase(runtime, rawInput = {}) {
  requireModule(runtime, "procurement");
  requireModule(runtime, "inventory");
  requireModule(runtime, "finance");
  const input = normalize(rawInput);
  return runtime.store.transact((tx) => {
    requireStore(tx, runtime.merchantId, input.store_id);
    const identity = {
      merchantId: runtime.merchantId,
      storeId: input.store_id,
      operation: "procurement.record",
      key: input.idempotency_key,
    };
    const replay = replayOrReject(tx, identity, input);
    if (replay.response) {
      return replay.response;
    }

    let totalMicro = 0;
    const stockUpdates = [];
    for (const item of input.items) {
      const product = requireProduct(
        tx,
        runtime.merchantId,
        input.store_id,
        item.product_id,
      );
      if (product.currency !== input.currency) {
        fail("CURRENCY_MISMATCH", `product ${product.id} uses another currency`, 409);
      }
      totalMicro = checkedAdd(
        totalMicro,
        checkedMultiply(item.quantity, item.unit_cost_micro, "purchase line total"),
        "purchase total",
      );
      const stock = tx.getInventory(runtime.merchantId, input.store_id, item.product_id);
      const nextQuantity = checkedAdd(stock.quantity, item.quantity, "inventory quantity");
      const updated = {
        ...stock,
        quantity: nextQuantity,
        revision: stock.revision + 1,
      };
      tx.putInventory(updated);
      stockUpdates.push({
        product_id: item.product_id,
        quantity: nextQuantity,
        revision: updated.revision,
      });
    }

    const createdAt = runtime.now();
    const purchaseId = runtime.createId("purchase");
    const journalId = runtime.createId("journal");
    tx.insertPurchase({
      id: purchaseId,
      merchant_id: runtime.merchantId,
      store_id: input.store_id,
      supplier_ref: input.supplier_ref,
      currency: input.currency,
      total_micro: totalMicro,
      items: input.items,
      created_at: createdAt,
    });
    tx.insertJournal({
      id: journalId,
      merchant_id: runtime.merchantId,
      store_id: input.store_id,
      source_type: "purchase",
      source_id: purchaseId,
      currency: input.currency,
      lines: [
        { account: "inventory_asset", direction: "debit", amount_micro: totalMicro },
        { account: input.credit_account, direction: "credit", amount_micro: totalMicro },
      ],
      created_at: createdAt,
    });
    const response = {
      purchase_id: purchaseId,
      journal_id: journalId,
      store_id: input.store_id,
      currency: input.currency,
      total_micro: totalMicro,
      inventory: stockUpdates,
      created_at: createdAt,
    };
    appendAudit(tx, runtime, input.store_id, "procurement.recorded", purchaseId, {
      currency: input.currency,
      total_micro: totalMicro,
      line_count: input.items.length,
    });
    saveIdempotency(tx, identity, replay.requestHash, response, createdAt);
    return response;
  });
}
