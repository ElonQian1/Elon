export const PORTABILITY_ARCHIVE_SCHEMA_V1:
  'open_commerce.consumer_portability_encrypted_archive.v1'
export const PORTABILITY_ARCHIVE_SCHEMA:
  'open_commerce.consumer_portability_encrypted_archive.v2'
export const PORTABILITY_ARCHIVE_ITERATIONS: 310000
export const PORTABILITY_ARCHIVE_MAX_PLAINTEXT_BYTES: number

export interface PortabilityEncryptedArchive {
  schema: typeof PORTABILITY_ARCHIVE_SCHEMA | typeof PORTABILITY_ARCHIVE_SCHEMA_V1
  kdf: {
    name: 'PBKDF2'
    hash: 'SHA-256'
    iterations: typeof PORTABILITY_ARCHIVE_ITERATIONS
    salt_base64: string
  }
  cipher: {
    name: 'AES-256-GCM'
    nonce_base64: string
    auth_tag_length_bits: 128
  }
  plaintext_sha256: string
  ciphertext_base64: string
  created_at: string
}

export function encryptPortabilityArchive(
  value: unknown,
  passphrase: string,
): Promise<PortabilityEncryptedArchive>
export function decryptPortabilityArchive(
  archive: PortabilityEncryptedArchive,
  passphrase: string,
): Promise<unknown>
export function isPortabilityEncryptedArchive(value: unknown): value is PortabilityEncryptedArchive
