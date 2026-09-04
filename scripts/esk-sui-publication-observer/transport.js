const https = require('node:https')
const dns = require('node:dns')
const { TextDecoder } = require('node:util')
const { ObservationError, endpoint, publicAddress } = require('./contract')

const MAX_BYTES = 128 * 1024
const TIMEOUT_MS = 12000

function resolveAddresses(host, _options, callback, resolver = new dns.Resolver(), timeoutMs = 4000) {
  let settled = false
  let pending = 2
  const records = []
  const finish = error => {
    if (settled) return
    settled = true
    clearTimeout(timer)
    resolver.cancel()
    callback(error, records)
  }
  const timer = setTimeout(() => finish(new ObservationError('TIMEOUT')), timeoutMs)
  try {
    for (const family of [4, 6]) {
      resolver[`resolve${family}`](host, (error, addresses) => {
        if (settled) return
        if (!error) records.push(...addresses.map(address => ({ address, family })))
        pending -= 1
        if (!pending) finish(records.length ? null : new ObservationError('NETWORK_ERROR'))
      })
    }
  } catch { finish(new ObservationError('NETWORK_ERROR')) }
}

function publicLookup(host, options, callback, resolve = resolveAddresses) {
  resolve(host, { all: true }, (error, records) => {
    if (error) return callback(error instanceof ObservationError ? error : new ObservationError('NETWORK_ERROR'))
    if (!Array.isArray(records) || !records.length || records.some(r => !publicAddress(r.address))) {
      return callback(new ObservationError('PRIVATE_ADDRESS'))
    }
    // Pin the validated address to this TLS request; do not resolve again after checking DNS.
    const record = records[0]
    if (options?.all) callback(null, [record])
    else callback(null, record.address, record.family)
  })
}

// makePayload is a trusted module's fixed-query factory, never a CLI-supplied query.
// Keep its evaluation after the synchronous endpoint/options guards.
function readGraphql(url, makePayload, options = {}) {
  endpoint(url)
  const request = options.request || https.request
  const timeout = options.timeoutMs ?? TIMEOUT_MS
  if (!Number.isInteger(timeout) || timeout < 1 || timeout > TIMEOUT_MS) {
    throw new ObservationError('INVALID_INPUT')
  }
  const body = JSON.stringify(makePayload())
  return new Promise((resolve, reject) => {
    let settled = false
    let req
    let response
    let timer
    const finish = (error, value) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      if (error) {
        response?.destroy()
        req?.destroy()
        reject(error)
      } else resolve(value)
    }
    // Covers DNS, connection, TLS and body, including a peer that never ends its response.
    timer = setTimeout(() => finish(new ObservationError('TIMEOUT')), timeout)
    try {
      req = request(url, {
        method: 'POST', agent: false, lookup: publicLookup, autoSelectFamily: false,
        rejectUnauthorized: true,
        headers: { 'content-type': 'application/json', accept: 'application/json',
          'accept-encoding': 'identity', 'content-length': Buffer.byteLength(body) },
      }, res => {
        response = res
        res.on('error', () => finish(new ObservationError('NETWORK_ERROR')))
        res.on('aborted', () => finish(new ObservationError('NETWORK_ERROR')))
        if (res.statusCode !== 200) return finish(new ObservationError('HTTP_ERROR'))
        const type = res.headers['content-type'] || ''
        if (!/^(application\/json|application\/graphql-response\+json)(;|$)/i.test(type) ||
            (res.headers['content-encoding'] && res.headers['content-encoding'] !== 'identity')) {
          return finish(new ObservationError('INVALID_RESPONSE'))
        }
        if (Number(res.headers['content-length']) > MAX_BYTES) {
          return finish(new ObservationError('RESPONSE_TOO_LARGE'))
        }
        let bytes = 0
        const chunks = []
        res.on('data', chunk => {
          if (settled) return
          bytes += chunk.length
          if (bytes > MAX_BYTES) return finish(new ObservationError('RESPONSE_TOO_LARGE'))
          chunks.push(chunk)
        })
        res.on('end', () => {
          if (settled) return
          try {
            const data = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(Buffer.concat(chunks)))
            if (!data || typeof data !== 'object' || Array.isArray(data)) {
              return finish(new ObservationError('INVALID_RESPONSE'))
            }
            if (data.errors !== undefined && (!Array.isArray(data.errors) || data.errors.length)) {
              return finish(new ObservationError('GRAPHQL_ERROR'))
            }
            finish(null, data.data)
          } catch { finish(new ObservationError('INVALID_RESPONSE')) }
        })
      })
      req.on('error', error => finish(error instanceof ObservationError ? error :
        new ObservationError('NETWORK_ERROR')))
      req.end(body)
    } catch { finish(new ObservationError('NETWORK_ERROR')) }
  })
}

module.exports = { MAX_BYTES, TIMEOUT_MS, resolveAddresses, publicLookup, readGraphql }
