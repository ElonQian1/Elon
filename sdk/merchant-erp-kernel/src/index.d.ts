export type ErpModuleKey = "catalog" | "inventory" | "order" | "procurement" | "finance";

export interface ErpStoreRecord {
  id: string;
  merchant_id: string;
  name: string;
  status?: "active" | "disabled";
}

export interface ErpProductRecord {
  id: string;
  merchant_id: string;
  store_id: string;
  sku: string;
  name: string;
  category?: string | null;
  currency: string;
  unit_price_micro: number;
  active?: boolean;
}

export interface ErpInventoryRecord {
  merchant_id: string;
  store_id: string;
  product_id: string;
  quantity: number;
  revision: number;
}

export interface ErpStorageTransaction {
  listStores(merchantId: string): ErpStoreRecord[];
  getStore(merchantId: string, storeId: string): ErpStoreRecord | null;
  searchProducts(merchantId: string, storeId: string, query: string, limit: number): ErpProductRecord[];
  getProduct(merchantId: string, storeId: string, productId: string): ErpProductRecord | null;
  getInventory(merchantId: string, storeId: string, productId: string): ErpInventoryRecord;
  putInventory(record: ErpInventoryRecord): void;
  getIdempotency(merchantId: string, storeId: string, operation: string, key: string): unknown;
  putIdempotency(record: unknown): void;
  insertPurchase(record: unknown): void;
  insertJournal(record: unknown): void;
  insertOrder(record: unknown): void;
  getOrder(merchantId: string, storeId: string, orderId: string): unknown;
  appendAudit(record: unknown): void;
}

export interface ErpStorageAdapter {
  read<T>(work: (transaction: ErpStorageTransaction) => T | Promise<T>): Promise<T>;
  transact<T>(work: (transaction: ErpStorageTransaction) => T | Promise<T>): Promise<T>;
}

export interface ErpKernelPlugin {
  key: string;
  enrichCatalogItem?(item: Record<string, unknown>): unknown | Promise<unknown>;
  validateOrder?(input: Record<string, unknown>): void | Promise<void>;
}

export interface MerchantErpKernelOptions {
  merchantId: string;
  store: ErpStorageAdapter;
  enabledModules?: ErpModuleKey[];
  plugins?: ErpKernelPlugin[];
  clock?: () => number | string | Date;
  idFactory?: () => string;
}

export interface MerchantErpKernel {
  readonly contract: "yilong.erp.kernel.v1";
  readonly merchantId: string;
  readonly enabledModules: readonly string[];
  readonly capabilities: readonly string[];
  listStores(): Promise<unknown>;
  searchCatalog(input: Record<string, unknown>): Promise<unknown>;
  queryInventory(input: Record<string, unknown>): Promise<unknown>;
  recordPurchase(input: Record<string, unknown>): Promise<unknown>;
  createOrder(input: Record<string, unknown>): Promise<unknown>;
  getOrder(input: Record<string, unknown>): Promise<unknown>;
  invoke(capabilityKey: string, input: Record<string, unknown>): Promise<unknown>;
}

export interface OpenCommerceProvider {
  readonly schema: "yilong.erp.open_commerce_provider.v1";
  readonly merchant_id: string;
  readonly capabilities: readonly Record<string, unknown>[];
  invoke(request: { capability_key: string; input: Record<string, unknown> }): Promise<unknown>;
}

export class ErpKernelError extends Error {
  code: string;
  status: number;
}

export class MemoryErpStore implements ErpStorageAdapter {
  constructor(seed?: Record<string, unknown[]>);
  read<T>(work: (transaction: ErpStorageTransaction) => T | Promise<T>): Promise<T>;
  transact<T>(work: (transaction: ErpStorageTransaction) => T | Promise<T>): Promise<T>;
  snapshot(): Promise<Record<string, unknown[]>>;
}

export function createMerchantErpKernel(options: MerchantErpKernelOptions): MerchantErpKernel;
export function createOpenCommerceProvider(kernel: MerchantErpKernel): OpenCommerceProvider;
export function createMerchantRuntimeBinding(provider: OpenCommerceProvider): {
  readonly schema: "yilong.erp.merchant_runtime_binding.v1";
  readonly merchantId: string;
  readonly capabilities: readonly Record<string, unknown>[];
  readonly handlers: Readonly<Record<string, (
    input: Record<string, unknown>,
    context: { merchantId: string; idempotencyKey: string },
  ) => Promise<unknown>>>;
};
