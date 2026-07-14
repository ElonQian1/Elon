interface RandomUuidCrypto {
  randomUUID?: () => string
}

export function createDeviceLeaseClientId(
  cryptoApi: RandomUuidCrypto | undefined = globalThis.crypto,
  now: () => number = Date.now,
  random: () => number = Math.random,
) {
  if (typeof cryptoApi?.randomUUID === 'function') {
    return `uit_${cryptoApi.randomUUID().replace(/-/g, '')}`
  }
  return `uit_${now().toString(36)}_${random().toString(36).slice(2, 12)}`
}
