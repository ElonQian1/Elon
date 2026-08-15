import { fail } from "./errors.js";
import { requireModule, requireStore } from "./runtime.js";
import { requireId, requireUniqueItems } from "./validation.js";

function normalizeQuery(value) {
  if (value === undefined || value === null) {
    return "";
  }
  if (typeof value !== "string" || value.trim().length > 120) {
    fail("INVALID_INPUT", "query must contain at most 120 characters");
  }
  return value.trim();
}

function normalizeLimit(value) {
  if (value === undefined) {
    return 20;
  }
  if (!Number.isSafeInteger(value) || value < 1 || value > 50) {
    fail("INVALID_INPUT", "limit must be an integer from 1 to 50");
  }
  return value;
}

async function enrichItem(runtime, item) {
  const extensions = {};
  for (const plugin of runtime.plugins) {
    if (!plugin.enrichCatalogItem) {
      continue;
    }
    const value = await plugin.enrichCatalogItem(structuredClone(item));
    if (value !== undefined) {
      extensions[plugin.key] = value;
    }
  }
  return Object.keys(extensions).length ? { ...item, extensions } : item;
}

export async function listStores(runtime) {
  return runtime.store.read((tx) => ({
    stores: tx
      .listStores(runtime.merchantId)
      .filter((store) => store.status !== "disabled")
      .map(({ id, name, status = "active" }) => ({ id, name, status })),
  }));
}

export async function searchCatalog(runtime, input = {}) {
  requireModule(runtime, "catalog");
  const storeId = requireId(input.store_id, "store_id");
  const query = normalizeQuery(input.query);
  const limit = normalizeLimit(input.limit);
  const base = await runtime.store.read((tx) => {
    requireStore(tx, runtime.merchantId, storeId);
    return tx.searchProducts(runtime.merchantId, storeId, query, limit).map((product) => {
      const stock = tx.getInventory(runtime.merchantId, storeId, product.id);
      return {
        product_id: product.id,
        sku: product.sku,
        name: product.name,
        category: product.category ?? null,
        currency: product.currency,
        unit_price_micro: product.unit_price_micro,
        available_quantity: stock.quantity,
      };
    });
  });
  return {
    store_id: storeId,
    items: await Promise.all(base.map((item) => enrichItem(runtime, item))),
  };
}

export async function queryInventory(runtime, input = {}) {
  requireModule(runtime, "inventory");
  const storeId = requireId(input.store_id, "store_id");
  requireUniqueItems(
    (input.product_ids ?? []).map((productId) => ({ product_id: productId })),
    "product_id",
    "product_ids",
  );
  return runtime.store.read((tx) => {
    requireStore(tx, runtime.merchantId, storeId);
    return {
      store_id: storeId,
      items: input.product_ids.map((productId) => {
        const product = tx.getProduct(runtime.merchantId, storeId, productId);
        const stock = tx.getInventory(runtime.merchantId, storeId, productId);
        return {
          product_id: productId,
          available: Boolean(product && product.active !== false),
          quantity: product ? stock.quantity : 0,
          revision: product ? stock.revision : 0,
        };
      }),
    };
  });
}
