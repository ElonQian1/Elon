import { createHmac, timingSafeEqual } from 'node:crypto'

export const DEVELOPER_WEBHOOK_EVENT_SCHEMA = 'open_commerce.developer_webhook_event.v1'
export const DEVELOPER_WEBHOOK_MAX_CLOCK_SKEW_SECONDS = 300

export class DeveloperWebhookSignatureError extends Error {
  constructor(code, message) {
    super(message)
    this.name = 'DeveloperWebhookSignatureError'
    this.code = code
  }
}

export function developerWebhookSignatureMessage(timestamp, body) {
  const timestampText = String(timestamp).trim()
  if (!/^\d{1,16}$/.test(timestampText)) {
    throw new DeveloperWebhookSignatureError('invalid_timestamp', 'webhook timestamp is invalid')
  }
  const bodyBytes = body instanceof Uint8Array ? Buffer.from(body) : Buffer.from(String(body), 'utf8')
  return Buffer.concat([Buffer.from(`${timestampText}.`, 'utf8'), bodyBytes])
}

export function verifyDeveloperWebhookSignature({
  secret,
  headers,
  body,
  nowUnix = Math.floor(Date.now() / 1000),
  maxClockSkewSeconds = DEVELOPER_WEBHOOK_MAX_CLOCK_SKEW_SECONDS,
}) {
  if (typeof secret !== 'string' || !secret.startsWith('whsec_') || secret.length < 40) {
    throw new DeveloperWebhookSignatureError('invalid_secret', 'webhook signing secret is invalid')
  }
  const eventId = requiredHeader(headers, 'x-yilong-webhook-id')
  const timestamp = requiredHeader(headers, 'x-yilong-webhook-timestamp')
  const signature = requiredHeader(headers, 'x-yilong-webhook-signature')
  if (!signature.startsWith('v1=')) {
    throw new DeveloperWebhookSignatureError('invalid_signature', 'webhook signature version is invalid')
  }
  const signatureHex = signature.slice(3)
  if (!/^[0-9a-f]{64}$/i.test(signatureHex)) {
    throw new DeveloperWebhookSignatureError('invalid_signature', 'webhook signature is invalid')
  }
  const timestampUnix = Number(timestamp)
  if (!Number.isSafeInteger(timestampUnix)) {
    throw new DeveloperWebhookSignatureError('invalid_timestamp', 'webhook timestamp is invalid')
  }
  if (!Number.isInteger(maxClockSkewSeconds) || maxClockSkewSeconds < 0 || maxClockSkewSeconds > 3600) {
    throw new DeveloperWebhookSignatureError('invalid_clock_skew', 'maxClockSkewSeconds is invalid')
  }
  if (Math.abs(nowUnix - timestampUnix) > maxClockSkewSeconds) {
    throw new DeveloperWebhookSignatureError('stale_timestamp', 'webhook timestamp is outside the allowed clock skew')
  }
  const expected = createHmac('sha256', secret)
    .update(developerWebhookSignatureMessage(timestamp, body))
    .digest()
  const actual = Buffer.from(signatureHex, 'hex')
  if (expected.length !== actual.length || !timingSafeEqual(expected, actual)) {
    throw new DeveloperWebhookSignatureError('signature_mismatch', 'webhook signature does not match')
  }
  return { eventId, timestampUnix }
}

function requiredHeader(headers, name) {
  let value
  if (headers && typeof headers.get === 'function') {
    value = headers.get(name)
  } else if (headers && typeof headers === 'object') {
    const entry = Object.entries(headers).find(([key]) => key.toLowerCase() === name)
    value = Array.isArray(entry?.[1]) ? entry[1][0] : entry?.[1]
  }
  if (typeof value !== 'string' || !value.trim()) {
    throw new DeveloperWebhookSignatureError('missing_header', `${name} header is required`)
  }
  return value.trim()
}
