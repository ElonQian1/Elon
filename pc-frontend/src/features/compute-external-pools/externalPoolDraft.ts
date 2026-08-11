import { type AdapterCapability } from './externalPoolApi'

export const REQUIRED_ADAPTER_CAPABILITIES = [
  'authenticated_ack',
  'authenticated_events',
  'cancel_no_start',
  'idempotent_commit',
  'prepare',
  'reconcile',
] as const

export function parseIdentifiers(value: string, label: string): string[] {
  const values = [...new Set(value.split(',').map((item) => item.trim()).filter(Boolean))].sort()
  if (!values.length) throw new Error(`${label}至少填写一项`)
  if (values.length > 64) throw new Error(`${label}最多填写 64 项`)
  if (values.some((item) => item.length > 80 || /[\u0000-\u001f\u007f]/.test(item))) {
    throw new Error(`${label}包含无效标识`)
  }
  return values
}

export function optionalDigest(value: string, label: string): string | null {
  const normalized = value.trim()
  if (!normalized) return null
  requireDigest(normalized, label)
  return normalized
}

export function requireDigest(value: string, label: string): string {
  const normalized = value.trim()
  if (!/^[0-9a-f]{64}$/.test(normalized)) throw new Error(`${label}必须为 64 位小写 SHA-256`)
  return normalized
}

export function optionalPair(first: string, second: string, label: string): [string | null, string | null] {
  const left = first.trim()
  const right = second.trim()
  if (Boolean(left) !== Boolean(right)) throw new Error(`${label}必须同时填写或同时留空`)
  return [left || null, right || null]
}

export function canonicalUtcNow(): string {
  return new Date().toISOString().replace(/\.(\d{3})Z$/, '.$1000000Z')
}

export function createRequestId(prefix: string): string {
  return `${prefix}-${createNonce()}`
}

export function createIdempotencyKey(prefix: string): string {
  return `${prefix}:${createNonce()}`
}

export function defaultCapabilities(): AdapterCapability[] {
  return REQUIRED_ADAPTER_CAPABILITIES.map((capability_id) => ({ capability_id, capability_revision: 1 }))
}

export function updateCapabilityRevision(
  capabilities: AdapterCapability[],
  index: number,
  revision: number,
): AdapterCapability[] {
  return capabilities.map((capability, capabilityIndex) => capabilityIndex === index
    ? { ...capability, capability_revision: revision }
    : capability)
}

export function requirePositiveInteger(value: number, label: string): number {
  if (!Number.isSafeInteger(value) || value <= 0) throw new Error(`${label}必须为正整数`)
  return value
}

function createNonce() {
  return typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.random().toString(16).slice(2)}`
}
