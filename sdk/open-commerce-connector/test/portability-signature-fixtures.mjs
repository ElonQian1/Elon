import { generateKeyPairSync } from 'node:crypto'

function rsaPair(modulusLength = 2048) {
  return generateKeyPairSync('rsa', {
    modulusLength,
    publicKeyEncoding: { type: 'spki', format: 'pem' },
    privateKeyEncoding: { type: 'pkcs8', format: 'pem' },
  })
}

export const primaryKeys = rsaPair()
export const secondaryKeys = rsaPair()

export function weakRsaKeys() {
  return rsaPair(1024)
}

export function portabilityPackage(overrides = {}) {
  return {
    schema: 'open_commerce.consumer_portability_export.v5',
    id: 'portability-fixture-1',
    source_project_id: 'consumer-project-fixture',
    idempotency_key: 'portable:fixture:1',
    payload_sha256: 'a'.repeat(64),
    payload_json: '{"schema":"fixture"}',
    payload: { schema: 'fixture' },
    created_at: '2026-08-10T08:00:00.000Z',
    ...overrides,
  }
}
