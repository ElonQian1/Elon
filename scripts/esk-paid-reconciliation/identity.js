const { createHash } = require('node:crypto')

function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort().map(key => `${JSON.stringify(key)}:${canonical(value[key])}`).join(',')}}`
  }
  return JSON.stringify(value)
}

const fingerprint = value => createHash('sha256').update(canonical(value), 'utf8').digest('hex')

function assetReference(value) {
  // Hex-address spelling is immaterial; Base58 and provider identifiers remain case-sensitive.
  return /^0x[0-9a-f]{1,64}$/i.test(value) ? `0x${value.slice(2).toLowerCase().padStart(64, '0')}` : value
}

function normalizedSource(source) {
  return { ...source, asset_reference: assetReference(source.asset_reference) }
}

function sourceFingerprint(source) {
  return fingerprint({ schema: 'yilong.payment_source.v1', ...normalizedSource(source) })
}

function paymentKey(source, row) {
  const reference = source.reference_format === 'hex32'
    ? row.external_payment_reference.replace(/^0x/i, '').toLowerCase()
    : row.external_payment_reference
  return fingerprint({
    schema: 'yilong.payment_identity.v1', namespace: source.namespace, network: source.network,
    asset_symbol: source.asset_symbol, asset_reference: assetReference(source.asset_reference),
    external_payment_reference: reference, transfer_index: row.transfer_index,
  })
}

module.exports = { canonical, fingerprint, normalizedSource, sourceFingerprint, paymentKey }
