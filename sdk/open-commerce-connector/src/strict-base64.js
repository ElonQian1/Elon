const CANONICAL_BASE64 =
  /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/

export function decodeStrictBase64(value, { label, minBytes = 0, maxBytes }) {
  if (
    typeof value !== 'string'
    || value.length % 4 !== 0
    || !CANONICAL_BASE64.test(value)
    || value.length > Math.ceil(maxBytes / 3) * 4
  ) {
    throw new TypeError(`${label} must be canonical Base64`)
  }
  const decoded = Buffer.from(value, 'base64')
  if (
    decoded.length < minBytes
    || decoded.length > maxBytes
    || decoded.toString('base64') !== value
  ) {
    throw new TypeError(`${label} has an invalid decoded length`)
  }
  return decoded
}
