const DEFINITIONS = Object.freeze({
  "catalog.search": {
    display_name: "Search catalog",
    access: "public",
    action: false,
    input_schema: {
      type: "object",
      additionalProperties: false,
      required: ["store_id"],
      properties: {
        store_id: { type: "string" },
        query: { type: "string", maxLength: 120 },
        limit: { type: "integer", minimum: 1, maximum: 50 },
      },
    },
  },
  "inventory.query": {
    display_name: "Query inventory",
    access: "authorized",
    action: false,
    input_schema: {
      type: "object",
      additionalProperties: false,
      required: ["store_id", "product_ids"],
      properties: {
        store_id: { type: "string" },
        product_ids: {
          type: "array",
          minItems: 1,
          maxItems: 100,
          items: { type: "string" },
        },
      },
    },
  },
  "order.create": {
    display_name: "Create unpaid order",
    access: "authorized",
    action: true,
    input_schema: {
      type: "object",
      additionalProperties: false,
      required: ["store_id", "idempotency_key", "items"],
      properties: {
        store_id: { type: "string" },
        idempotency_key: { type: "string" },
        customer_ref: { type: "string", maxLength: 240 },
        items: {
          type: "array",
          minItems: 1,
          maxItems: 100,
          items: {
            type: "object",
            additionalProperties: false,
            required: ["product_id", "quantity"],
            properties: {
              product_id: { type: "string" },
              quantity: { type: "integer", minimum: 1, maximum: 100000 },
            },
          },
        },
      },
    },
  },
  "order.status": {
    display_name: "Read order status",
    access: "authorized",
    action: false,
    input_schema: {
      type: "object",
      additionalProperties: false,
      required: ["store_id", "order_id"],
      properties: {
        store_id: { type: "string" },
        order_id: { type: "string" },
      },
    },
  },
});

export function createOpenCommerceProvider(kernel) {
  if (kernel?.contract !== "yilong.erp.kernel.v1" || typeof kernel.invoke !== "function") {
    throw new TypeError("kernel must implement yilong.erp.kernel.v1");
  }
  const capabilities = kernel.capabilities.map((capabilityKey) => ({
    capability_key: capabilityKey,
    ...structuredClone(DEFINITIONS[capabilityKey]),
  }));
  return Object.freeze({
    schema: "yilong.erp.open_commerce_provider.v1",
    capabilities: Object.freeze(capabilities),
    invoke: ({ capability_key: capabilityKey, input }) => kernel.invoke(capabilityKey, input),
  });
}
