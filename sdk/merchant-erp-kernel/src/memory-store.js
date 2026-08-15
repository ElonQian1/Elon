import { fail } from "./errors.js";

function clone(value) {
  return structuredClone(value);
}

function composite(...values) {
  return values.join("\u001f");
}

function seedMap(values, keyOf) {
  return new Map((values ?? []).map((value) => [keyOf(value), clone(value)]));
}

function createState(seed) {
  return {
    stores: seedMap(seed.stores, (value) => composite(value.merchant_id, value.id)),
    products: seedMap(seed.products, (value) =>
      composite(value.merchant_id, value.store_id, value.id),
    ),
    inventory: seedMap(seed.inventory, (value) =>
      composite(value.merchant_id, value.store_id, value.product_id),
    ),
    purchases: seedMap(seed.purchases, (value) =>
      composite(value.merchant_id, value.store_id, value.id),
    ),
    journals: seedMap(seed.journals, (value) =>
      composite(value.merchant_id, value.store_id, value.id),
    ),
    orders: seedMap(seed.orders, (value) =>
      composite(value.merchant_id, value.store_id, value.id),
    ),
    idempotency: seedMap(seed.idempotency, (value) =>
      composite(value.merchant_id, value.store_id, value.operation, value.key),
    ),
    audit: clone(seed.audit ?? []),
  };
}

class MemoryTransaction {
  constructor(state) {
    this.state = state;
  }

  listStores(merchantId) {
    return [...this.state.stores.values()]
      .filter((store) => store.merchant_id === merchantId)
      .sort((left, right) => left.id.localeCompare(right.id))
      .map(clone);
  }

  getStore(merchantId, storeId) {
    return clone(this.state.stores.get(composite(merchantId, storeId)) ?? null);
  }

  searchProducts(merchantId, storeId, query, limit) {
    const normalized = query.toLocaleLowerCase();
    return [...this.state.products.values()]
      .filter(
        (product) =>
          product.merchant_id === merchantId &&
          product.store_id === storeId &&
          product.active !== false &&
          (!normalized ||
            product.name.toLocaleLowerCase().includes(normalized) ||
            product.sku.toLocaleLowerCase().includes(normalized) ||
            (product.category ?? "").toLocaleLowerCase().includes(normalized)),
      )
      .sort((left, right) => left.name.localeCompare(right.name) || left.id.localeCompare(right.id))
      .slice(0, limit)
      .map(clone);
  }

  getProduct(merchantId, storeId, productId) {
    return clone(
      this.state.products.get(composite(merchantId, storeId, productId)) ?? null,
    );
  }

  getInventory(merchantId, storeId, productId) {
    return clone(
      this.state.inventory.get(composite(merchantId, storeId, productId)) ?? {
        merchant_id: merchantId,
        store_id: storeId,
        product_id: productId,
        quantity: 0,
        revision: 0,
      },
    );
  }

  putInventory(record) {
    this.state.inventory.set(
      composite(record.merchant_id, record.store_id, record.product_id),
      clone(record),
    );
  }

  getIdempotency(merchantId, storeId, operation, key) {
    return clone(
      this.state.idempotency.get(composite(merchantId, storeId, operation, key)) ?? null,
    );
  }

  putIdempotency(record) {
    this.state.idempotency.set(
      composite(record.merchant_id, record.store_id, record.operation, record.key),
      clone(record),
    );
  }

  insertPurchase(record) {
    const key = composite(record.merchant_id, record.store_id, record.id);
    if (this.state.purchases.has(key)) {
      fail("RECORD_CONFLICT", "purchase already exists", 409);
    }
    this.state.purchases.set(key, clone(record));
  }

  insertJournal(record) {
    const key = composite(record.merchant_id, record.store_id, record.id);
    if (this.state.journals.has(key)) {
      fail("RECORD_CONFLICT", "journal already exists", 409);
    }
    this.state.journals.set(key, clone(record));
  }

  insertOrder(record) {
    const key = composite(record.merchant_id, record.store_id, record.id);
    if (this.state.orders.has(key)) {
      fail("RECORD_CONFLICT", "order already exists", 409);
    }
    this.state.orders.set(key, clone(record));
  }

  getOrder(merchantId, storeId, orderId) {
    return clone(this.state.orders.get(composite(merchantId, storeId, orderId)) ?? null);
  }

  appendAudit(record) {
    this.state.audit.push(clone(record));
  }
}

export class MemoryErpStore {
  #state;
  #tail = Promise.resolve();

  constructor(seed = {}) {
    this.#state = createState(seed);
  }

  async read(work) {
    await this.#tail;
    return work(new MemoryTransaction(clone(this.#state)));
  }

  async transact(work) {
    const execute = async () => {
      const candidate = clone(this.#state);
      const result = await work(new MemoryTransaction(candidate));
      this.#state = candidate;
      return clone(result);
    };
    const operation = this.#tail.then(execute, execute);
    this.#tail = operation.then(
      () => undefined,
      () => undefined,
    );
    return operation;
  }

  async snapshot() {
    await this.#tail;
    const state = clone(this.#state);
    return {
      stores: [...state.stores.values()],
      products: [...state.products.values()],
      inventory: [...state.inventory.values()],
      purchases: [...state.purchases.values()],
      journals: [...state.journals.values()],
      orders: [...state.orders.values()],
      idempotency: [...state.idempotency.values()],
      audit: state.audit,
    };
  }
}
