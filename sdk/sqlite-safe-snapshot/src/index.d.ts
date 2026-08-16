export declare const SQLITE_SAFE_SNAPSHOT_RECEIPT_SCHEMA:
  "yilong.sqlite_safe_snapshot.receipt.v1";

export declare class SqliteSafeSnapshotError extends Error {
  readonly code: string;
  constructor(code: string, message: string);
}

export interface SqliteSnapshotRequirements {
  expectedUserVersion?: number | null;
  requiredTables?: string[];
}

export interface CreateSqliteSnapshotOptions extends SqliteSnapshotRequirements {
  sourcePath: string;
  destinationPath: string;
}

export interface VerifySqliteSnapshotOptions extends SqliteSnapshotRequirements {
  path: string;
  expectedSha256?: string | null;
}

export interface RestoreSqliteSnapshotOptions extends SqliteSnapshotRequirements {
  sourcePath: string;
  destinationPath: string;
  expectedSha256: string;
}

export interface SqliteSnapshotReceipt {
  readonly schema: typeof SQLITE_SAFE_SNAPSHOT_RECEIPT_SCHEMA;
  readonly operation: "snapshot" | "verify" | "restore";
  readonly sha256: string;
  readonly size_bytes: number;
  readonly sqlite_user_version: number;
  readonly tables: readonly string[];
  readonly pages_copied: number;
  readonly created_at_ms: number;
  readonly source_sha256?: string;
}

export declare function createSqliteSnapshot(
  options: CreateSqliteSnapshotOptions,
): Promise<SqliteSnapshotReceipt>;

export declare function verifySqliteSnapshot(
  options: VerifySqliteSnapshotOptions,
): Promise<SqliteSnapshotReceipt>;

export declare function restoreSqliteSnapshot(
  options: RestoreSqliteSnapshotOptions,
): Promise<SqliteSnapshotReceipt>;
