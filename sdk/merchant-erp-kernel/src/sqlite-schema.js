import { ErpKernelError } from "./errors.js";

export const SQLITE_ERP_SCHEMA_VERSION = 1;

const CREATE_SCHEMA_V1 = `
CREATE TABLE yilong_erp_records (
  kind TEXT NOT NULL CHECK (
    kind IN ('store', 'product', 'inventory', 'purchase', 'journal', 'order', 'idempotency')
  ),
  merchant_id TEXT NOT NULL,
  store_id TEXT NOT NULL,
  record_key TEXT NOT NULL,
  payload_json TEXT NOT NULL CHECK (json_valid(payload_json)),
  PRIMARY KEY (kind, merchant_id, store_id, record_key)
) STRICT;

CREATE INDEX yilong_erp_records_scope
  ON yilong_erp_records (merchant_id, store_id, kind, record_key);

CREATE TABLE yilong_erp_audit (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  merchant_id TEXT NOT NULL,
  store_id TEXT NOT NULL,
  record_key TEXT NOT NULL,
  payload_json TEXT NOT NULL CHECK (json_valid(payload_json)),
  UNIQUE (merchant_id, store_id, record_key)
) STRICT;

CREATE INDEX yilong_erp_audit_scope
  ON yilong_erp_audit (merchant_id, store_id, sequence);
`;

function requireInteger(value, field, minimum, maximum) {
  if (!Number.isInteger(value) || value < minimum || value > maximum) {
    throw new ErpKernelError(
      "INVALID_STORAGE_OPTION",
      `${field} must be an integer from ${minimum} to ${maximum}`,
    );
  }
  return value;
}

function readUserVersion(database) {
  const row = database.prepare("PRAGMA user_version").get();
  const version = Number(row?.user_version);
  if (!Number.isInteger(version) || version < 0) {
    throw new ErpKernelError("STORAGE_CORRUPTED", "SQLite schema version is invalid", 500);
  }
  return version;
}

function migrateFromEmpty(database) {
  database.exec("BEGIN IMMEDIATE");
  try {
    database.exec(CREATE_SCHEMA_V1);
    database.exec(`PRAGMA user_version = ${SQLITE_ERP_SCHEMA_VERSION}`);
    database.exec("COMMIT");
  } catch (error) {
    try {
      database.exec("ROLLBACK");
    } catch {
      // Preserve the original migration failure.
    }
    throw error;
  }
}

function verifySchemaV1(database) {
  const expectedColumns = {
    yilong_erp_records: ["kind", "merchant_id", "store_id", "record_key", "payload_json"],
    yilong_erp_audit: ["sequence", "merchant_id", "store_id", "record_key", "payload_json"],
  };
  for (const [table, expected] of Object.entries(expectedColumns)) {
    const actual = database
      .prepare(`PRAGMA table_info(${table})`)
      .all()
      .map((row) => String(row.name));
    if (actual.length !== expected.length || actual.some((name, index) => name !== expected[index])) {
      throw new ErpKernelError(
        "STORAGE_CORRUPTED",
        "SQLite ERP schema is incomplete",
        500,
      );
    }
  }
}

export function initializeSqliteErpSchema(database, options = {}) {
  const busyTimeoutMs = requireInteger(
    options.busyTimeoutMs ?? 5_000,
    "busyTimeoutMs",
    0,
    60_000,
  );
  database.exec("PRAGMA foreign_keys = ON");
  database.exec(`PRAGMA busy_timeout = ${busyTimeoutMs}`);
  database.exec("PRAGMA journal_mode = WAL");

  const version = readUserVersion(database);
  if (version > SQLITE_ERP_SCHEMA_VERSION) {
    throw new ErpKernelError(
      "UNSUPPORTED_SCHEMA_VERSION",
      `SQLite ERP schema ${version} is newer than supported version ${SQLITE_ERP_SCHEMA_VERSION}`,
      409,
    );
  }
  if (version === 0) {
    migrateFromEmpty(database);
  }
  verifySchemaV1(database);
  return SQLITE_ERP_SCHEMA_VERSION;
}
