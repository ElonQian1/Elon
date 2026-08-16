import { DatabaseSync } from 'node:sqlite'

export const MERCHANT_RUNTIME_SQLITE_STORE_SCHEMA =
  'merchant_runtime.sqlite_idempotency_store.v1'
export const MERCHANT_RUNTIME_SQLITE_STORE_SCHEMA_VERSION = 1

const META_TABLE = 'yilong_merchant_runtime_idempotency_meta'
const RECORD_TABLE = 'yilong_merchant_runtime_idempotency'
const RESULT_MAX_BYTES = 1024 * 1024
const KEY_FIELDS = Object.freeze([
  'merchantId',
  'requesterAppId',
  'capabilityKey',
  'idempotencyKey',
])

export class MerchantRuntimeSqliteStoreError extends Error {
  constructor(code, message) {
    super(message)
    this.name = 'MerchantRuntimeSqliteStoreError'
    this.code = code
  }
}

export function createSqliteMerchantRuntimeIdempotencyStore(options = {}) {
  if (!options || typeof options !== 'object' || Array.isArray(options)) {
    throw new TypeError('options must be an object')
  }
  const path = requireText(options.path, 'options.path', 32_768)
  const takeoverAfterMs = boundedInteger(
    options.takeoverAfterMs ?? 60_000,
    1_000,
    600_000,
    'options.takeoverAfterMs',
  )
  const busyTimeoutMs = boundedInteger(
    options.busyTimeoutMs ?? 5_000,
    0,
    30_000,
    'options.busyTimeoutMs',
  )
  const clock = options.clock ?? Date.now
  if (typeof clock !== 'function') throw new TypeError('options.clock must be a function')

  let database
  try {
    database = new DatabaseSync(path)
    initialize(database, busyTimeoutMs)
  } catch (error) {
    try {
      database?.close()
    } catch {
      // Preserve the initialization failure.
    }
    throw storageError(error)
  }
  let closed = false

  function assertOpen() {
    if (closed) {
      throw new MerchantRuntimeSqliteStoreError(
        'SQLITE_STORE_CLOSED',
        'SQLite merchant runtime idempotency storage is closed',
      )
    }
  }

  function now() {
    const value = clock()
    if (!Number.isSafeInteger(value) || value < 0) {
      throw new TypeError('options.clock must return a non-negative safe integer')
    }
    return value
  }

  return Object.freeze({
    schema: MERCHANT_RUNTIME_SQLITE_STORE_SCHEMA,

    async claim(input) {
      assertOpen()
      const normalized = normalizeInput(input)
      const timestamp = now()
      return transaction(database, () => {
        const current = selectRecord(database, normalized)
        if (!current) {
          insertProcessing(database, normalized, timestamp)
          return { status: 'claimed' }
        }
        if (current.request_hash !== normalized.requestHash) {
          return { status: 'conflict' }
        }
        if (current.status === 'succeeded') {
          return { status: 'replayed', result: decodeResult(current.result_json) }
        }
        if (current.status !== 'processing' || !Number.isSafeInteger(current.updated_at_ms)) {
          throw corrupted()
        }
        if (timestamp - current.updated_at_ms < takeoverAfterMs) {
          return { status: 'busy' }
        }
        replaceOwner(database, normalized, timestamp)
        return { status: 'claimed' }
      })
    },

    async complete(input, result) {
      assertOpen()
      const normalized = normalizeInput(input)
      const encoded = encodeResult(result)
      const timestamp = now()
      return transaction(database, () => {
        const update = database.prepare(
          `UPDATE ${RECORD_TABLE}
              SET status = 'succeeded', result_json = ?, updated_at_ms = ?
            WHERE merchant_id = ? AND requester_app_id = ?
              AND capability_key = ? AND idempotency_key = ?
              AND status = 'processing' AND invocation_id = ? AND request_hash = ?`,
        ).run(
          encoded,
          timestamp,
          ...keyValues(normalized),
          normalized.invocationId,
          normalized.requestHash,
        )
        return Number(update.changes) === 1
      })
    },

    async release(input) {
      assertOpen()
      const normalized = normalizeInput(input)
      transaction(database, () => {
        database.prepare(
          `DELETE FROM ${RECORD_TABLE}
            WHERE merchant_id = ? AND requester_app_id = ?
              AND capability_key = ? AND idempotency_key = ?
              AND status = 'processing' AND invocation_id = ? AND request_hash = ?`,
        ).run(
          ...keyValues(normalized),
          normalized.invocationId,
          normalized.requestHash,
        )
      })
    },

    close() {
      if (closed) return
      try {
        database.close()
        closed = true
      } catch (error) {
        throw storageError(error)
      }
    },
  })
}

function initialize(database, busyTimeoutMs) {
  database.exec(`PRAGMA busy_timeout = ${busyTimeoutMs}`)
  database.exec('PRAGMA foreign_keys = ON')
  database.exec('PRAGMA journal_mode = WAL')
  database.exec('PRAGMA synchronous = FULL')
  transaction(database, () => {
    database.exec(`
      CREATE TABLE IF NOT EXISTS ${META_TABLE} (
        key TEXT PRIMARY KEY NOT NULL,
        value TEXT NOT NULL
      ) STRICT;
    `)
    const version = database.prepare(
      `SELECT value FROM ${META_TABLE} WHERE key = 'schema_version'`,
    ).get()
    if (!version) {
      database.prepare(
        `INSERT INTO ${META_TABLE} (key, value) VALUES ('schema_version', ?)`,
      ).run(String(MERCHANT_RUNTIME_SQLITE_STORE_SCHEMA_VERSION))
    } else if (version.value !== String(MERCHANT_RUNTIME_SQLITE_STORE_SCHEMA_VERSION)) {
      throw new MerchantRuntimeSqliteStoreError(
        'SQLITE_SCHEMA_UNSUPPORTED',
        'SQLite merchant runtime idempotency schema version is unsupported',
      )
    }
    database.exec(`
      CREATE TABLE IF NOT EXISTS ${RECORD_TABLE} (
        merchant_id TEXT NOT NULL,
        requester_app_id TEXT NOT NULL,
        capability_key TEXT NOT NULL,
        idempotency_key TEXT NOT NULL,
        invocation_id TEXT NOT NULL,
        request_hash TEXT NOT NULL,
        status TEXT NOT NULL CHECK (status IN ('processing', 'succeeded')),
        result_json TEXT,
        updated_at_ms INTEGER NOT NULL CHECK (updated_at_ms >= 0),
        PRIMARY KEY (merchant_id, requester_app_id, capability_key, idempotency_key),
        CHECK (
          (status = 'processing' AND result_json IS NULL)
          OR (status = 'succeeded' AND result_json IS NOT NULL)
        )
      ) STRICT;
    `)
  })
}

function transaction(database, operation) {
  let active = false
  try {
    database.exec('BEGIN IMMEDIATE')
    active = true
    const result = operation()
    database.exec('COMMIT')
    active = false
    return result
  } catch (error) {
    if (active) {
      try {
        database.exec('ROLLBACK')
      } catch {
        // Preserve the operation failure.
      }
    }
    throw storageError(error)
  }
}

function selectRecord(database, input) {
  return database.prepare(
    `SELECT invocation_id, request_hash, status, result_json, updated_at_ms
       FROM ${RECORD_TABLE}
      WHERE merchant_id = ? AND requester_app_id = ?
        AND capability_key = ? AND idempotency_key = ?`,
  ).get(...keyValues(input))
}

function insertProcessing(database, input, timestamp) {
  database.prepare(
    `INSERT INTO ${RECORD_TABLE}
      (merchant_id, requester_app_id, capability_key, idempotency_key,
       invocation_id, request_hash, status, result_json, updated_at_ms)
     VALUES (?, ?, ?, ?, ?, ?, 'processing', NULL, ?)`,
  ).run(
    ...keyValues(input),
    input.invocationId,
    input.requestHash,
    timestamp,
  )
}

function replaceOwner(database, input, timestamp) {
  const update = database.prepare(
    `UPDATE ${RECORD_TABLE}
        SET invocation_id = ?, status = 'processing', result_json = NULL, updated_at_ms = ?
      WHERE merchant_id = ? AND requester_app_id = ?
        AND capability_key = ? AND idempotency_key = ? AND request_hash = ?`,
  ).run(
    input.invocationId,
    timestamp,
    ...keyValues(input),
    input.requestHash,
  )
  if (Number(update.changes) !== 1) throw corrupted()
}

function normalizeInput(input) {
  if (!input || typeof input !== 'object' || Array.isArray(input)) {
    throw new TypeError('idempotency input must be an object')
  }
  const normalized = {}
  for (const field of [...KEY_FIELDS, 'invocationId']) {
    normalized[field] = requireText(input[field], `input.${field}`, 512)
  }
  if (typeof input.requestHash !== 'string' || !/^[a-f0-9]{64}$/.test(input.requestHash)) {
    throw new TypeError('input.requestHash must be a lowercase SHA-256 digest')
  }
  normalized.requestHash = input.requestHash
  return normalized
}

function keyValues(input) {
  return KEY_FIELDS.map((field) => input[field])
}

function encodeResult(result) {
  if (!result || typeof result !== 'object' || Array.isArray(result)) {
    throw new TypeError('result must be an object')
  }
  try {
    const encoded = JSON.stringify(result)
    if (typeof encoded !== 'string') throw new Error('not serializable')
    if (Buffer.byteLength(encoded, 'utf8') > RESULT_MAX_BYTES) throw new Error('too large')
    return encoded
  } catch {
    throw new MerchantRuntimeSqliteStoreError(
      'SQLITE_RESULT_INVALID',
      'Merchant runtime idempotency result must be JSON serializable',
    )
  }
}

function decodeResult(value) {
  try {
    const decoded = JSON.parse(String(value))
    if (!decoded || typeof decoded !== 'object' || Array.isArray(decoded)) throw new Error()
    return decoded
  } catch {
    throw corrupted()
  }
}

function requireText(value, field, maximum) {
  if (typeof value !== 'string' || value.length === 0 || value.length > maximum) {
    throw new TypeError(`${field} must be a non-empty string up to ${maximum} characters`)
  }
  return value
}

function boundedInteger(value, minimum, maximum, field) {
  if (!Number.isInteger(value) || value < minimum || value > maximum) {
    throw new TypeError(`${field} must be an integer between ${minimum} and ${maximum}`)
  }
  return value
}

function corrupted() {
  return new MerchantRuntimeSqliteStoreError(
    'SQLITE_STORE_CORRUPTED',
    'SQLite merchant runtime idempotency storage contains an invalid record',
  )
}

function storageError(error) {
  if (error instanceof MerchantRuntimeSqliteStoreError || error instanceof TypeError) return error
  if (
    error?.code === 'ERR_SQLITE_ERROR'
    && (error.errcode === 5 || error.errcode === 6 || /locked|busy/i.test(error.message ?? ''))
  ) {
    return new MerchantRuntimeSqliteStoreError(
      'SQLITE_STORE_BUSY',
      'SQLite merchant runtime idempotency storage is busy',
    )
  }
  return new MerchantRuntimeSqliteStoreError(
    'SQLITE_STORE_FAILURE',
    'SQLite merchant runtime idempotency storage operation failed',
  )
}
