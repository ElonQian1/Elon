import { randomUUID } from "node:crypto";

import { sha256 } from "./canonical.js";
import { fail } from "./errors.js";
import { requireId } from "./validation.js";

export const DEFAULT_MODULES = Object.freeze([
  "catalog",
  "inventory",
  "order",
  "procurement",
  "finance",
]);

export function createRuntime(options) {
  if (!options?.store?.read || !options?.store?.transact) {
    fail("INVALID_STORAGE_ADAPTER", "store must implement read and transact");
  }
  const merchantId = requireId(options.merchantId, "merchantId");
  const enabledModules = new Set(options.enabledModules ?? DEFAULT_MODULES);
  const plugins = options.plugins ?? [];
  const pluginKeys = new Set();
  for (const plugin of plugins) {
    const key = requireId(plugin?.key, "plugin.key");
    if (pluginKeys.has(key)) {
      fail("INVALID_PLUGIN", `duplicate plugin ${key}`);
    }
    pluginKeys.add(key);
  }
  return {
    merchantId,
    store: options.store,
    enabledModules,
    plugins,
    now: () => new Date((options.clock ?? Date.now)()).toISOString(),
    createId: (prefix) => `${prefix}_${(options.idFactory ?? randomUUID)()}`,
  };
}

export function requireModule(runtime, moduleKey) {
  if (!runtime.enabledModules.has(moduleKey)) {
    fail("MODULE_DISABLED", `ERP module ${moduleKey} is disabled`, 409);
  }
}

export function requireStore(tx, merchantId, storeId) {
  const store = tx.getStore(merchantId, storeId);
  if (!store || store.status === "disabled") {
    fail("STORE_NOT_FOUND", "store is unavailable", 404);
  }
  return store;
}

export function requireProduct(tx, merchantId, storeId, productId) {
  const product = tx.getProduct(merchantId, storeId, productId);
  if (!product || product.active === false) {
    fail("PRODUCT_NOT_FOUND", `product ${productId} is unavailable`, 404);
  }
  return product;
}

export function replayOrReject(tx, identity, request) {
  const requestHash = sha256(request);
  const previous = tx.getIdempotency(
    identity.merchantId,
    identity.storeId,
    identity.operation,
    identity.key,
  );
  if (!previous) {
    return { requestHash, response: null };
  }
  if (previous.request_hash !== requestHash) {
    fail("IDEMPOTENCY_CONFLICT", "idempotency key was used with different input", 409);
  }
  return { requestHash, response: previous.response };
}

export function saveIdempotency(tx, identity, requestHash, response, createdAt) {
  tx.putIdempotency({
    merchant_id: identity.merchantId,
    store_id: identity.storeId,
    operation: identity.operation,
    key: identity.key,
    request_hash: requestHash,
    response,
    created_at: createdAt,
  });
}

export function appendAudit(tx, runtime, storeId, action, subjectId, metadata = {}) {
  tx.appendAudit({
    id: runtime.createId("audit"),
    merchant_id: runtime.merchantId,
    store_id: storeId,
    action,
    subject_id: subjectId,
    metadata,
    created_at: runtime.now(),
  });
}
