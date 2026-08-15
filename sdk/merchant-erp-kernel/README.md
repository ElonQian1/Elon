# Merchant ERP Kernel

`@yilong/merchant-erp-kernel` is the headless reusable runtime behind merchant-owned ERP projects.
It contains business rules, not UI, database credentials, deployment code, or platform-specific APIs.

Merchant projects provide:

- a storage adapter implementing atomic `read` and `transact` boundaries;
- their own UI and theme;
- private plugins for catalog enrichment or order validation;
- deployment and secret management in the merchant-controlled environment.

The V1 kernel provides store discovery, catalog search, inventory reads, atomic purchase posting,
balanced inventory accounting journals, unpaid order creation, order status, idempotency, and audit
records. `createOpenCommerceProvider` exposes the compatible consumer-AI capability surface without
claiming payment, fulfillment, or an external-platform connection.

```js
import {
  MemoryErpStore,
  createMerchantErpKernel,
  createOpenCommerceProvider,
} from "@yilong/merchant-erp-kernel";

const storage = new MemoryErpStore({ stores: [], products: [], inventory: [] });
const kernel = createMerchantErpKernel({ merchantId: "merchant_demo", store: storage });
const provider = createOpenCommerceProvider(kernel);
```

`MemoryErpStore` is a deterministic test and local-development adapter. Production merchant systems
must implement `ErpStorageAdapter` with their own PostgreSQL, SQLite, or service transaction layer.
