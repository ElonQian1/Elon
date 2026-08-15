import { listStores, queryInventory, searchCatalog } from "./catalog.js";
import { fail } from "./errors.js";
import { createOrder, getOrder } from "./orders.js";
import { recordPurchase } from "./procurement.js";
import { createRuntime } from "./runtime.js";

const CAPABILITY_MODULES = Object.freeze({
  "catalog.search": ["catalog"],
  "inventory.query": ["inventory"],
  "order.create": ["catalog", "inventory", "order"],
  "order.status": ["order"],
});

function availableCapabilities(runtime) {
  return Object.entries(CAPABILITY_MODULES)
    .filter(([, modules]) => modules.every((moduleKey) => runtime.enabledModules.has(moduleKey)))
    .map(([capabilityKey]) => capabilityKey);
}

export function createMerchantErpKernel(options) {
  const runtime = createRuntime(options);
  const capabilities = availableCapabilities(runtime);
  return Object.freeze({
    contract: "yilong.erp.kernel.v1",
    merchantId: runtime.merchantId,
    enabledModules: Object.freeze([...runtime.enabledModules].sort()),
    capabilities: Object.freeze(capabilities),
    listStores: () => listStores(runtime),
    searchCatalog: (input) => searchCatalog(runtime, input),
    queryInventory: (input) => queryInventory(runtime, input),
    recordPurchase: (input) => recordPurchase(runtime, input),
    createOrder: (input) => createOrder(runtime, input),
    getOrder: (input) => getOrder(runtime, input),
    invoke: (capabilityKey, input) => {
      if (!capabilities.includes(capabilityKey)) {
        fail("CAPABILITY_UNAVAILABLE", `capability ${capabilityKey} is unavailable`, 404);
      }
      switch (capabilityKey) {
        case "catalog.search":
          return searchCatalog(runtime, input);
        case "inventory.query":
          return queryInventory(runtime, input);
        case "order.create":
          return createOrder(runtime, input);
        case "order.status":
          return getOrder(runtime, input);
        default:
          fail("CAPABILITY_UNAVAILABLE", `capability ${capabilityKey} is unavailable`, 404);
      }
    },
  });
}
