const { CurrencyObservationError, objectId } = require('./contract')

// Offline only: no Sui client, wallet, signer, transaction builder or RPC is instantiated.
async function deriveCurrencyId(coinType) {
  try {
    const { deriveObjectID } = await import('@mysten/sui/utils')
    // CurrencyKey<T> is fieldless: Move inserts dummy_field=false, encoded as one zero byte.
    return objectId(deriveObjectID('0xc', `0x2::coin_registry::CurrencyKey<${coinType}>`,
      new Uint8Array([0])))
  } catch {
    throw new CurrencyObservationError('SDK_UNAVAILABLE')
  }
}

module.exports = { deriveCurrencyId }
