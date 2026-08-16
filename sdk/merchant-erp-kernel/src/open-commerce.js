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
    merchant_id: kernel.merchantId,
    capabilities: Object.freeze(capabilities),
    invoke: ({ capability_key: capabilityKey, input }) => kernel.invoke(capabilityKey, input),
  });
}

export function createMerchantRuntimeBinding(provider) {
  if (
    provider?.schema !== "yilong.erp.open_commerce_provider.v1" ||
    typeof provider.merchant_id !== "string" ||
    typeof provider.invoke !== "function" ||
    !Array.isArray(provider.capabilities)
  ) {
    throw new TypeError("provider must implement yilong.erp.open_commerce_provider.v1");
  }
  const merchantId = provider.merchant_id;
  const capabilities = provider.capabilities.map((capability) =>
    toRuntimeCapability(capability),
  );
  const handlers = Object.fromEntries(
    provider.capabilities.map((capability) => [
      capability.capability_key,
      async (input, context) => {
        if (context?.merchantId !== merchantId) {
          throw new Error("merchant runtime identity does not match ERP provider");
        }
        const effectiveInput = capability.capability_key === "order.create"
          ? { ...input, idempotency_key: context.idempotencyKey }
          : input;
        const result = await provider.invoke({
          capability_key: capability.capability_key,
          input: effectiveInput,
        });
        return withBusinessReceipt(capability.capability_key, result);
      },
    ]),
  );
  return Object.freeze({
    schema: "yilong.erp.merchant_runtime_binding.v1",
    merchantId,
    capabilities: Object.freeze(capabilities),
    handlers: Object.freeze(handlers),
  });
}

function toRuntimeCapability(capability) {
  const inputSchema = structuredClone(capability.input_schema);
  if (capability.capability_key === "order.create") {
    delete inputSchema.properties?.idempotency_key;
    if (Array.isArray(inputSchema.required)) {
      inputSchema.required = inputSchema.required.filter(
        (field) => field !== "idempotency_key",
      );
    }
  }
  return Object.freeze({
    key: capability.capability_key,
    access: capability.access,
    action: capability.action === true,
    input_schema: inputSchema,
  });
}

function withBusinessReceipt(capabilityKey, result) {
  if (!capabilityKey.startsWith("order.") || !result || typeof result !== "object") {
    return result;
  }
  if (
    typeof result.order_id !== "string" ||
    typeof result.status !== "string" ||
    typeof result.created_at !== "string"
  ) {
    throw new TypeError("ERP order result cannot produce a standard business receipt");
  }
  return {
    ...result,
    _yilong_business_receipt: {
      schema: "open_commerce.merchant_business_receipt.v1",
      entity_type: "order",
      reference_id: result.order_id,
      state: result.status,
      occurred_at: result.created_at,
    },
  };
}
