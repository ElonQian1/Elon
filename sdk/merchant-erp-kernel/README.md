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
that already own a database can implement `ErpStorageAdapter` with their PostgreSQL or service
transaction layer.

## Merchant-owned SQLite

Node 22.13 and newer projects can use the optional built-in SQLite adapter without adding a native
third-party package. Node 22.5 through 22.12 require the `--experimental-sqlite` flag. Importing the
core entry does not load `node:sqlite`, so Node 20 projects can continue using the kernel with
another adapter.

```js
import { createMerchantErpKernel } from "@yilong/merchant-erp-kernel";
import { SqliteErpStore } from "@yilong/merchant-erp-kernel/sqlite";

const storage = new SqliteErpStore({
  path: "./erp.sqlite3",
  busyTimeoutMs: 5_000,
});
const kernel = createMerchantErpKernel({ merchantId: "merchant_demo", store: storage });

try {
  await kernel.listStores();
} finally {
  await storage.close();
}
```

The adapter owns a versioned schema, `BEGIN IMMEDIATE` write transactions, deterministic snapshots,
and structured `STORAGE_BUSY` failures. Keep one adapter instance per database in a process. SQLite
still coordinates other processes through its file lock; this adapter is not a multi-primary or
distributed database and does not add payment, fulfillment, deployment, or platform credentials.
