import { v4 as uuid } from 'uuid'
import { sha256 } from '@noble/hashes/sha2.js'
import { bytesToHex } from '@noble/hashes/utils.js'

// The Win workbench can use an HTTP origin. getRandomValues remains available
// there, while randomUUID and subtle are secure-context-only browser APIs.
export function gridReadRequestId(): string { return uuid().replace(/-/g, '') }

// Preserve the existing content-addressed SHA-256 key across all environments.
// Never fall back to a new random key for a retry of the same public snapshot.
export function gridShareDigest(document: unknown): string {
  return bytesToHex(sha256(new TextEncoder().encode(JSON.stringify(document))))
}
