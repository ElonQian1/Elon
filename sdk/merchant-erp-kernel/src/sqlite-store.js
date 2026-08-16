import { DatabaseSync } from "node:sqlite";

import { ErpKernelError } from "./errors.js";
import {
  initializeSqliteErpSchema,
  SQLITE_ERP_SCHEMA_VERSION,
} from "./sqlite-schema.js";

const RECORD_KINDS = Object.freeze([
  "store",
  "product",
  "inventory",
  "purchase",
  "journal",
  "order",
  "idempotency",
]);
const COLLECTION_BY_KIND = Object.freeze({
  store: "stores",
  product: "products",
  inventory: "inventory",
  purchase: "purchases",
  journal: "journals",
  order: "orders",
  idempotency: "idempotency",
});

function clone(value) {
  return structuredClone(value);
}

function requireText(value, field) {
  if (typeof value !== "string" || !value) {
    throw new ErpKernelError("INVALID_STORAGE_RECORD", `${field} must be a non-empty string`);
  }
  return value;
}

function encode(value) {
  try {
    const encoded = JSON.stringify(value);
    if (typeof encoded !== "string") throw new Error("not serializable");
    return encoded;
  } catch {
    throw new ErpKernelError("INVALID_STORAGE_RECORD", "ERP record must be JSON serializable");
  }
}

function decode(value) {
  try {
    return JSON.parse(String(value));
  } catch {
    throw new ErpKernelError("STORAGE_CORRUPTED", "SQLite ERP record is invalid", 500);
  }
}

function isSqliteError(error) {
  return error?.code === "ERR_SQLITE_ERROR" || Number.isInteger(error?.errcode);
}

function storageError(error) {
  if (error instanceof ErpKernelError) return error;
  if (!isSqliteError(error)) return error;
  if (error.errcode === 5 || error.errcode === 6 || /locked|busy/i.test(error.message ?? "")) {
    return new ErpKernelError("STORAGE_BUSY", "SQLite ERP storage is busy", 503);
  }
  return new ErpKernelError("STORAGE_FAILURE", "SQLite ERP storage operation failed", 500);
}

function recordConflict(error, subject) {
  if (
    isSqliteError(error) &&
    (error.errcode === 19 || /constraint|unique/i.test(error.message ?? ""))
  ) {
    return new ErpKernelError("RECORD_CONFLICT", `${subject} already exists`, 409);
  }
  return storageError(error);
}

function idempotencyKey(operation, key) {
  return JSON.stringify([operation, key]);
}

function hasSeed(seed) {
  if (!seed || typeof seed !== "object" || Array.isArray(seed)) {
    throw new ErpKernelError("INVALID_STORAGE_OPTION", "seed must be an object");
  }
  let populated = false;
  for (const collection of [...Object.values(COLLECTION_BY_KIND), "audit"]) {
    const values = seed[collection];
    if (values === undefined) continue;
    if (!Array.isArray(values)) {
      throw new ErpKernelError("INVALID_STORAGE_OPTION", `${collection} seed must be an array`);
    }
    populated ||= values.length > 0;
  }
  return populated;
}

class SqliteTransaction {
  #database;
  #writable;
  #active = true;

  constructor(database, writable) {
    this.#database = database;
    this.#writable = writable;
  }

  close() {
    this.#active = false;
  }

  #assertActive() {
    if (!this.#active) {
      throw new ErpKernelError("TRANSACTION_CLOSED", "ERP storage transaction is closed", 409);
    }
  }

  #assertWritable() {
    this.#assertActive();
    if (!this.#writable) {
      throw new ErpKernelError(
        "READ_ONLY_TRANSACTION",
        "ERP read transaction cannot change storage",
        409,
      );
    }
  }

  #readRecord(kind, merchantId, storeId, recordKey) {
    this.#assertActive();
    try {
      const row = this.#database
        .prepare(
          `SELECT payload_json
             FROM yilong_erp_records
            WHERE kind = ? AND merchant_id = ? AND store_id = ? AND record_key = ?`,
        )
        .get(kind, merchantId, storeId, recordKey);
      return row ? decode(row.payload_json) : null;
    } catch (error) {
      throw storageError(error);
    }
  }

  #listRecords(kind, merchantId = null, storeId = null) {
    this.#assertActive();
    let sql = "SELECT payload_json FROM yilong_erp_records WHERE kind = ?";
    const parameters = [kind];
    if (merchantId !== null) {
      sql += " AND merchant_id = ?";
      parameters.push(merchantId);
    }
    if (storeId !== null) {
      sql += " AND store_id = ?";
      parameters.push(storeId);
    }
    sql += " ORDER BY merchant_id, store_id, record_key";
    try {
      return this.#database
        .prepare(sql)
        .all(...parameters)
        .map((row) => decode(row.payload_json));
    } catch (error) {
      throw storageError(error);
    }
  }

  #insert(kind, merchantId, storeId, recordKey, record, subject) {
    this.#assertWritable();
    try {
      this.#database
        .prepare(
          `INSERT INTO yilong_erp_records
             (kind, merchant_id, store_id, record_key, payload_json)
           VALUES (?, ?, ?, ?, ?)`,
        )
        .run(kind, merchantId, storeId, recordKey, encode(record));
    } catch (error) {
      throw recordConflict(error, subject);
    }
  }

  #upsert(kind, merchantId, storeId, recordKey, record) {
    this.#assertWritable();
    try {
      this.#database
        .prepare(
          `INSERT INTO yilong_erp_records
             (kind, merchant_id, store_id, record_key, payload_json)
           VALUES (?, ?, ?, ?, ?)
           ON CONFLICT (kind, merchant_id, store_id, record_key)
           DO UPDATE SET payload_json = excluded.payload_json`,
        )
        .run(kind, merchantId, storeId, recordKey, encode(record));
    } catch (error) {
      throw storageError(error);
    }
  }

  listStores(merchantId) {
    return this.#listRecords("store", merchantId).sort((left, right) =>
      left.id.localeCompare(right.id),
    );
  }

  getStore(merchantId, storeId) {
    return this.#readRecord("store", merchantId, storeId, storeId);
  }

  searchProducts(merchantId, storeId, query, limit) {
    const normalized = query.toLocaleLowerCase();
    return this.#listRecords("product", merchantId, storeId)
      .filter(
        (product) =>
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
    return this.#readRecord("product", merchantId, storeId, productId);
  }

  getInventory(merchantId, storeId, productId) {
    return (
      this.#readRecord("inventory", merchantId, storeId, productId) ?? {
        merchant_id: merchantId,
        store_id: storeId,
        product_id: productId,
        quantity: 0,
        revision: 0,
      }
    );
  }

  putInventory(record) {
    const merchantId = requireText(record?.merchant_id, "inventory.merchant_id");
    const storeId = requireText(record?.store_id, "inventory.store_id");
    const productId = requireText(record?.product_id, "inventory.product_id");
    this.#upsert("inventory", merchantId, storeId, productId, record);
  }

  getIdempotency(merchantId, storeId, operation, key) {
    return this.#readRecord(
      "idempotency",
      merchantId,
      storeId,
      idempotencyKey(operation, key),
    );
  }

  putIdempotency(record) {
    const merchantId = requireText(record?.merchant_id, "idempotency.merchant_id");
    const storeId = requireText(record?.store_id, "idempotency.store_id");
    const operation = requireText(record?.operation, "idempotency.operation");
    const key = requireText(record?.key, "idempotency.key");
    this.#insert(
      "idempotency",
      merchantId,
      storeId,
      idempotencyKey(operation, key),
      record,
      "idempotency record",
    );
  }

  insertPurchase(record) {
    this.#insertRecord("purchase", record);
  }

  insertJournal(record) {
    this.#insertRecord("journal", record);
  }

  insertOrder(record) {
    this.#insertRecord("order", record);
  }

  #insertRecord(kind, record) {
    const merchantId = requireText(record?.merchant_id, `${kind}.merchant_id`);
    const storeId = requireText(record?.store_id, `${kind}.store_id`);
    const id = requireText(record?.id, `${kind}.id`);
    this.#insert(kind, merchantId, storeId, id, record, kind);
  }

  getOrder(merchantId, storeId, orderId) {
    return this.#readRecord("order", merchantId, storeId, orderId);
  }

  appendAudit(record) {
    this.#assertWritable();
    const merchantId = requireText(record?.merchant_id, "audit.merchant_id");
    const storeId = requireText(record?.store_id, "audit.store_id");
    const id = requireText(record?.id, "audit.id");
    try {
      this.#database
        .prepare(
          `INSERT INTO yilong_erp_audit
             (merchant_id, store_id, record_key, payload_json)
           VALUES (?, ?, ?, ?)`,
        )
        .run(merchantId, storeId, id, encode(record));
    } catch (error) {
      throw recordConflict(error, "audit record");
    }
  }

  seed(seed) {
    this.#assertWritable();
    for (const record of seed.stores ?? []) {
      const merchantId = requireText(record?.merchant_id, "store.merchant_id");
      const id = requireText(record?.id, "store.id");
      this.#insert("store", merchantId, id, id, record, "store");
    }
    for (const record of seed.products ?? []) {
      const merchantId = requireText(record?.merchant_id, "product.merchant_id");
      const storeId = requireText(record?.store_id, "product.store_id");
      const id = requireText(record?.id, "product.id");
      this.#insert("product", merchantId, storeId, id, record, "product");
    }
    for (const record of seed.inventory ?? []) this.putInventory(record);
    for (const record of seed.purchases ?? []) this.insertPurchase(record);
    for (const record of seed.journals ?? []) this.insertJournal(record);
    for (const record of seed.orders ?? []) this.insertOrder(record);
    for (const record of seed.idempotency ?? []) this.putIdempotency(record);
    for (const record of seed.audit ?? []) this.appendAudit(record);
  }

  snapshot() {
    this.#assertActive();
    const result = {};
    for (const kind of RECORD_KINDS) {
      result[COLLECTION_BY_KIND[kind]] = this.#listRecords(kind);
    }
    try {
      result.audit = this.#database
        .prepare("SELECT payload_json FROM yilong_erp_audit ORDER BY sequence")
        .all()
        .map((row) => decode(row.payload_json));
      return result;
    } catch (error) {
      throw storageError(error);
    }
  }
}

function databaseHasBusinessRecords(database) {
  const records = Number(database.prepare("SELECT COUNT(*) AS count FROM yilong_erp_records").get().count);
  const audit = Number(database.prepare("SELECT COUNT(*) AS count FROM yilong_erp_audit").get().count);
  return records > 0 || audit > 0;
}

function applySeed(database, seed) {
  if (!hasSeed(seed)) return;
  database.exec("BEGIN IMMEDIATE");
  const transaction = new SqliteTransaction(database, true);
  try {
    if (databaseHasBusinessRecords(database)) {
      throw new ErpKernelError(
        "SEED_REQUIRES_EMPTY_DATABASE",
        "SQLite ERP seed requires an empty business database",
        409,
      );
    }
    transaction.seed(seed);
    transaction.close();
    database.exec("COMMIT");
  } catch (error) {
    transaction.close();
    try {
      database.exec("ROLLBACK");
    } catch {
      // Preserve the original seed failure.
    }
    throw storageError(error);
  }
}

export class SqliteErpStore {
  #database;
  #tail = Promise.resolve();
  #accepting = true;
  #closePromise = null;

  constructor(options = {}) {
    const databasePath = options.path;
    if (typeof databasePath !== "string" || !databasePath.trim()) {
      throw new ErpKernelError("INVALID_STORAGE_OPTION", "path must be a non-empty string");
    }
    try {
      this.#database = new DatabaseSync(databasePath);
      initializeSqliteErpSchema(this.#database, {
        busyTimeoutMs: options.busyTimeoutMs,
      });
      applySeed(this.#database, options.seed ?? {});
    } catch (error) {
      try {
        this.#database?.close();
      } catch {
        // Preserve the original open or migration failure.
      }
      throw storageError(error);
    }
  }

  get schemaVersion() {
    return SQLITE_ERP_SCHEMA_VERSION;
  }

  #assertOpen() {
    if (!this.#accepting) {
      throw new ErpKernelError("STORAGE_CLOSED", "SQLite ERP storage is closed", 409);
    }
  }

  #enqueue(work) {
    this.#assertOpen();
    const operation = this.#tail.then(work, work);
    this.#tail = operation.then(
      () => undefined,
      () => undefined,
    );
    return operation;
  }

  #run(writable, work) {
    return this.#enqueue(async () => {
      let began = false;
      const transaction = new SqliteTransaction(this.#database, writable);
      try {
        this.#database.exec(writable ? "BEGIN IMMEDIATE" : "BEGIN");
        began = true;
        const result = await work(transaction);
        transaction.close();
        this.#database.exec("COMMIT");
        return clone(result);
      } catch (error) {
        transaction.close();
        if (began) {
          try {
            this.#database.exec("ROLLBACK");
          } catch {
            // Preserve the original transaction failure.
          }
        }
        throw storageError(error);
      }
    });
  }

  read(work) {
    if (typeof work !== "function") {
      throw new ErpKernelError("INVALID_STORAGE_WORK", "read requires a callback");
    }
    return this.#run(false, work);
  }

  transact(work) {
    if (typeof work !== "function") {
      throw new ErpKernelError("INVALID_STORAGE_WORK", "transact requires a callback");
    }
    return this.#run(true, work);
  }

  snapshot() {
    return this.read((transaction) => transaction.snapshot());
  }

  close() {
    if (this.#closePromise) return this.#closePromise;
    this.#accepting = false;
    this.#closePromise = this.#tail.then(() => {
      this.#database.close();
    });
    return this.#closePromise;
  }
}

export { SQLITE_ERP_SCHEMA_VERSION };
