import type {
  MerchantRuntimeIdempotencyInput,
  MerchantRuntimeIdempotencyStore,
} from "./index.js"

export const MERCHANT_RUNTIME_SQLITE_STORE_SCHEMA: "merchant_runtime.sqlite_idempotency_store.v1"
export const MERCHANT_RUNTIME_SQLITE_STORE_SCHEMA_VERSION: 1

export type MerchantRuntimeSqliteStoreErrorCode =
  | "SQLITE_RESULT_INVALID"
  | "SQLITE_SCHEMA_UNSUPPORTED"
  | "SQLITE_STORE_BUSY"
  | "SQLITE_STORE_CLOSED"
  | "SQLITE_STORE_CORRUPTED"
  | "SQLITE_STORE_FAILURE"

export class MerchantRuntimeSqliteStoreError extends Error {
  code: MerchantRuntimeSqliteStoreErrorCode
  constructor(code: MerchantRuntimeSqliteStoreErrorCode, message: string)
}

export interface SqliteMerchantRuntimeIdempotencyStore
  extends MerchantRuntimeIdempotencyStore {
  readonly schema: typeof MERCHANT_RUNTIME_SQLITE_STORE_SCHEMA
  close(): void
}

export function createSqliteMerchantRuntimeIdempotencyStore(options: {
  path: string
  takeoverAfterMs?: number
  busyTimeoutMs?: number
  clock?: () => number
}): SqliteMerchantRuntimeIdempotencyStore

export type { MerchantRuntimeIdempotencyInput }
