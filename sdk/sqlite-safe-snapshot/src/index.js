import { createHash, randomUUID } from "node:crypto";
import { createReadStream } from "node:fs";
import { link, lstat, open, unlink } from "node:fs/promises";
import { backup as sqliteBackup, DatabaseSync } from "node:sqlite";
import { basename, dirname, isAbsolute, join, resolve } from "node:path";

export const SQLITE_SAFE_SNAPSHOT_RECEIPT_SCHEMA =
  "yilong.sqlite_safe_snapshot.receipt.v1";

const MAX_TABLES = 128;
const MAX_TABLE_NAME_LENGTH = 256;
const MAX_SQLITE_USER_VERSION = 2_147_483_647;

export class SqliteSafeSnapshotError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "SqliteSafeSnapshotError";
    this.code = code;
  }
}

export async function createSqliteSnapshot(options = {}) {
  const normalized = normalizeCopyOptions(options, false);
  return publishDatabaseCopy({
    ...normalized,
    operation: "snapshot",
  });
}

export async function verifySqliteSnapshot(options = {}) {
  const normalized = normalizeVerifyOptions(options, false);
  await assertSourceFile(normalized.path);
  const inspection = inspectDatabase(normalized.path, normalized);
  const digest = await hashFile(normalized.path);
  if (normalized.expectedSha256 && digest.sha256 !== normalized.expectedSha256) {
    throw snapshotError(
      "SNAPSHOT_HASH_MISMATCH",
      "SQLite snapshot does not match the expected SHA-256 digest",
    );
  }
  return createReceipt("verify", digest, inspection, 0);
}

export async function restoreSqliteSnapshot(options = {}) {
  const normalized = normalizeCopyOptions(options, true);
  const verifiedSource = await verifySqliteSnapshot({
    path: normalized.sourcePath,
    expectedSha256: normalized.expectedSha256,
    expectedUserVersion: normalized.expectedUserVersion,
    requiredTables: normalized.requiredTables,
  });
  const restored = await publishDatabaseCopy({
    ...normalized,
    operation: "restore",
    beforePublish: async () => {
      const current = await hashFile(normalized.sourcePath);
      if (current.sha256 !== normalized.expectedSha256) {
        throw snapshotError(
          "SNAPSHOT_HASH_MISMATCH",
          "SQLite snapshot changed before restore publication",
        );
      }
    },
  });
  return Object.freeze({
    ...restored,
    source_sha256: verifiedSource.sha256,
  });
}

async function publishDatabaseCopy(options) {
  await assertSourceFile(options.sourcePath);
  await assertTargetAvailable(options.destinationPath);
  const temporaryPath = join(
    dirname(options.destinationPath),
    `.${basename(options.destinationPath)}.${randomUUID()}.sqlite-snapshot-tmp`,
  );
  let sourceDatabase;
  let published = false;
  try {
    sourceDatabase = new DatabaseSync(options.sourcePath, { readOnly: true });
    const pagesCopied = await sqliteBackup(sourceDatabase, temporaryPath);
    sourceDatabase.close();
    sourceDatabase = undefined;

    const inspection = inspectDatabase(temporaryPath, options);
    const digest = await hashFile(temporaryPath);
    await syncFile(temporaryPath);
    await options.beforePublish?.();
    try {
      await link(temporaryPath, options.destinationPath);
      published = true;
    } catch (error) {
      if (error?.code === "EEXIST") {
        throw snapshotError(
          "SNAPSHOT_TARGET_EXISTS",
          "SQLite snapshot target already exists",
        );
      }
      throw error;
    }
    await unlink(temporaryPath);
    return createReceipt(options.operation, digest, inspection, pagesCopied);
  } catch (error) {
    try {
      sourceDatabase?.close();
    } catch {
      // Preserve the operation failure.
    }
    await removeTemporaryFile(temporaryPath);
    throw storageError(error, published);
  }
}

function inspectDatabase(path, requirements) {
  let database;
  try {
    database = new DatabaseSync(path, { readOnly: true });
    const quickCheckRows = database.prepare("PRAGMA quick_check").all();
    const quickCheck = quickCheckRows.map((row) => String(Object.values(row)[0]));
    if (quickCheck.length !== 1 || quickCheck[0] !== "ok") {
      throw snapshotError(
        "SNAPSHOT_INTEGRITY_FAILED",
        "SQLite snapshot failed quick_check",
      );
    }
    const versionRow = database.prepare("PRAGMA user_version").get();
    const userVersion = Number(versionRow?.user_version);
    if (!Number.isInteger(userVersion) || userVersion < 0) {
      throw snapshotError(
        "SNAPSHOT_INTEGRITY_FAILED",
        "SQLite snapshot has an invalid user_version",
      );
    }
    if (
      requirements.expectedUserVersion !== null &&
      userVersion !== requirements.expectedUserVersion
    ) {
      throw snapshotError(
        "SNAPSHOT_SCHEMA_MISMATCH",
        "SQLite snapshot user_version does not match the required version",
      );
    }
    const tables = database
      .prepare(
        `SELECT name
           FROM sqlite_master
          WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
          ORDER BY name`,
      )
      .all()
      .map((row) => String(row.name));
    const available = new Set(tables);
    if (requirements.requiredTables.some((table) => !available.has(table))) {
      throw snapshotError(
        "SNAPSHOT_SCHEMA_MISMATCH",
        "SQLite snapshot is missing a required table",
      );
    }
    return Object.freeze({ userVersion, tables: Object.freeze(tables) });
  } catch (error) {
    throw storageError(error, false);
  } finally {
    try {
      database?.close();
    } catch {
      // The earlier inspection result remains authoritative.
    }
  }
}

function normalizeCopyOptions(options, requireExpectedHash) {
  requireObject(options, "options");
  const sourcePath = requireAbsolutePath(options.sourcePath, "options.sourcePath");
  const destinationPath = requireAbsolutePath(
    options.destinationPath,
    "options.destinationPath",
  );
  if (pathKey(sourcePath) === pathKey(destinationPath)) {
    throw snapshotError(
      "SNAPSHOT_PATH_CONFLICT",
      "SQLite snapshot source and target must be different files",
    );
  }
  return {
    sourcePath,
    destinationPath,
    ...normalizeRequirements(options, requireExpectedHash),
  };
}

function normalizeVerifyOptions(options, requireExpectedHash) {
  requireObject(options, "options");
  return {
    path: requireAbsolutePath(options.path, "options.path"),
    ...normalizeRequirements(options, requireExpectedHash),
  };
}

function normalizeRequirements(options, requireExpectedHash) {
  const requiredTables = normalizeRequiredTables(options.requiredTables ?? []);
  const expectedUserVersion = normalizeUserVersion(options.expectedUserVersion);
  const expectedSha256 = normalizeSha256(options.expectedSha256, requireExpectedHash);
  return { requiredTables, expectedUserVersion, expectedSha256 };
}

function normalizeRequiredTables(value) {
  if (!Array.isArray(value) || value.length > MAX_TABLES) {
    throw new TypeError(`requiredTables must be an array of at most ${MAX_TABLES} names`);
  }
  const normalized = value.map((table) => {
    if (
      typeof table !== "string" ||
      table.length === 0 ||
      table.length > MAX_TABLE_NAME_LENGTH ||
      table.includes("\0")
    ) {
      throw new TypeError("requiredTables contains an invalid table name");
    }
    return table;
  });
  return Object.freeze([...new Set(normalized)].sort());
}

function normalizeUserVersion(value) {
  if (value === undefined || value === null) return null;
  if (!Number.isInteger(value) || value < 0 || value > MAX_SQLITE_USER_VERSION) {
    throw new TypeError(
      `expectedUserVersion must be an integer from 0 to ${MAX_SQLITE_USER_VERSION}`,
    );
  }
  return value;
}

function normalizeSha256(value, required) {
  if (value === undefined || value === null || value === "") {
    if (required) {
      throw new TypeError("expectedSha256 is required for restore");
    }
    return null;
  }
  if (typeof value !== "string" || !/^[a-f0-9]{64}$/.test(value)) {
    throw new TypeError("expectedSha256 must be a lowercase SHA-256 digest");
  }
  return value;
}

function requireAbsolutePath(value, field) {
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    value.includes("\0") ||
    value === ":memory:" ||
    !isAbsolute(value)
  ) {
    throw new TypeError(`${field} must be an absolute on-disk path`);
  }
  return resolve(value);
}

function requireObject(value, field) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`${field} must be an object`);
  }
}

async function assertSourceFile(path) {
  let metadata;
  try {
    metadata = await lstat(path);
  } catch (error) {
    if (error?.code === "ENOENT") {
      throw snapshotError("SNAPSHOT_SOURCE_NOT_FOUND", "SQLite snapshot source was not found");
    }
    throw error;
  }
  if (metadata.isSymbolicLink()) {
    throw snapshotError(
      "SNAPSHOT_SYMBOLIC_LINK_REJECTED",
      "SQLite snapshot source cannot be a symbolic link",
    );
  }
  if (!metadata.isFile()) {
    throw snapshotError(
      "SNAPSHOT_SOURCE_NOT_FILE",
      "SQLite snapshot source must be a regular file",
    );
  }
}

async function assertTargetAvailable(path) {
  let parent;
  try {
    parent = await lstat(dirname(path));
  } catch (error) {
    if (error?.code === "ENOENT") {
      throw snapshotError(
        "SNAPSHOT_TARGET_PARENT_MISSING",
        "SQLite snapshot target directory does not exist",
      );
    }
    throw error;
  }
  if (parent.isSymbolicLink()) {
    throw snapshotError(
      "SNAPSHOT_SYMBOLIC_LINK_REJECTED",
      "SQLite snapshot target parent cannot be a symbolic link",
    );
  }
  if (!parent.isDirectory()) {
    throw snapshotError(
      "SNAPSHOT_TARGET_PARENT_INVALID",
      "SQLite snapshot target parent must be a directory",
    );
  }
  try {
    await lstat(path);
    throw snapshotError(
      "SNAPSHOT_TARGET_EXISTS",
      "SQLite snapshot target already exists",
    );
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
}

async function hashFile(path) {
  const hash = createHash("sha256");
  let bytes = 0;
  for await (const chunk of createReadStream(path)) {
    hash.update(chunk);
    bytes += chunk.length;
  }
  if (!Number.isSafeInteger(bytes)) {
    throw snapshotError("SNAPSHOT_TOO_LARGE", "SQLite snapshot byte length is unsafe");
  }
  return Object.freeze({ sha256: hash.digest("hex"), sizeBytes: bytes });
}

async function syncFile(path) {
  const handle = await open(path, "r+");
  try {
    await handle.sync();
  } finally {
    await handle.close();
  }
}

async function removeTemporaryFile(path) {
  try {
    await unlink(path);
  } catch (error) {
    if (error?.code !== "ENOENT") {
      // A leftover private temp file is safer than deleting an unverified path.
    }
  }
}

function createReceipt(operation, digest, inspection, pagesCopied) {
  return Object.freeze({
    schema: SQLITE_SAFE_SNAPSHOT_RECEIPT_SCHEMA,
    operation,
    sha256: digest.sha256,
    size_bytes: digest.sizeBytes,
    sqlite_user_version: inspection.userVersion,
    tables: inspection.tables,
    pages_copied: pagesCopied,
    created_at_ms: Date.now(),
  });
}

function pathKey(path) {
  return process.platform === "win32" ? path.toLocaleLowerCase("en-US") : path;
}

function snapshotError(code, message) {
  return new SqliteSafeSnapshotError(code, message);
}

function storageError(error, published) {
  if (error instanceof SqliteSafeSnapshotError || error instanceof TypeError) return error;
  if (error?.code === "ENOENT") {
    return snapshotError("SNAPSHOT_IO_FAILURE", "SQLite snapshot file disappeared");
  }
  if (error?.code === "EEXIST") {
    return snapshotError("SNAPSHOT_TARGET_EXISTS", "SQLite snapshot target already exists");
  }
  if (error?.code === "ERR_SQLITE_ERROR" || Number.isInteger(error?.errcode)) {
    return snapshotError(
      "SNAPSHOT_SQLITE_INVALID",
      "SQLite snapshot operation failed database validation",
    );
  }
  return snapshotError(
    published ? "SNAPSHOT_CLEANUP_FAILURE" : "SNAPSHOT_IO_FAILURE",
    published
      ? "SQLite snapshot was published but temporary cleanup failed"
      : "SQLite snapshot operation failed before publication",
  );
}
