import type { ErpStorageAdapter, ErpStorageTransaction } from "./index.js";

export interface SqliteErpSeed {
  stores?: Record<string, unknown>[];
  products?: Record<string, unknown>[];
  inventory?: Record<string, unknown>[];
  purchases?: Record<string, unknown>[];
  journals?: Record<string, unknown>[];
  orders?: Record<string, unknown>[];
  idempotency?: Record<string, unknown>[];
  audit?: Record<string, unknown>[];
}

export interface SqliteErpStoreOptions {
  path: string;
  busyTimeoutMs?: number;
  seed?: SqliteErpSeed;
}

export declare const SQLITE_ERP_SCHEMA_VERSION: 1;

export declare class SqliteErpStore implements ErpStorageAdapter {
  constructor(options: SqliteErpStoreOptions);
  readonly schemaVersion: number;
  read<T>(work: (transaction: ErpStorageTransaction) => T | Promise<T>): Promise<T>;
  transact<T>(work: (transaction: ErpStorageTransaction) => T | Promise<T>): Promise<T>;
  snapshot(): Promise<Record<string, unknown[]>>;
  close(): Promise<void>;
}
