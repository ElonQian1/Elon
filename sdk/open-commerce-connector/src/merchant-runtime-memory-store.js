export function createMemoryMerchantRuntimeIdempotencyStore({ takeoverAfterMs = 60_000 } = {}) {
  if (!Number.isInteger(takeoverAfterMs) || takeoverAfterMs < 1_000 || takeoverAfterMs > 600_000) {
    throw new TypeError('takeoverAfterMs must be an integer between 1000 and 600000')
  }
  const records = new Map()

  return Object.freeze({
    async claim(input) {
      const key = recordKey(input)
      const current = records.get(key)
      const now = Date.now()
      if (!current) {
        records.set(key, processing(input, now))
        return { status: 'claimed' }
      }
      if (current.requestHash !== input.requestHash) {
        return { status: 'conflict' }
      }
      if (current.status === 'succeeded') {
        return { status: 'replayed', result: structuredClone(current.result) }
      }
      if (now - current.updatedAt < takeoverAfterMs) {
        return { status: 'busy' }
      }
      records.set(key, processing(input, now))
      return { status: 'claimed' }
    },

    async complete(input, result) {
      const key = recordKey(input)
      const current = records.get(key)
      if (!ownedBy(current, input)) return false
      records.set(key, {
        ...current,
        status: 'succeeded',
        result: structuredClone(result),
        updatedAt: Date.now(),
      })
      return true
    },

    async release(input) {
      const key = recordKey(input)
      const current = records.get(key)
      if (ownedBy(current, input)) records.delete(key)
    },
  })
}

function recordKey(input) {
  return JSON.stringify([
    input.merchantId,
    input.requesterAppId,
    input.capabilityKey,
    input.idempotencyKey,
  ])
}

function processing(input, timestamp) {
  return {
    status: 'processing',
    invocationId: input.invocationId,
    requestHash: input.requestHash,
    updatedAt: timestamp,
  }
}

function ownedBy(record, input) {
  return record?.status === 'processing'
    && record.invocationId === input.invocationId
    && record.requestHash === input.requestHash
}
