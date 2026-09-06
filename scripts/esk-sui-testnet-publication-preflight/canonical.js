'use strict'

const { createHash } = require('node:crypto')
const { fail, isPlainObject } = require('./contract')

function canonicalJson(value) {
  if (value === null || typeof value === 'boolean' || typeof value === 'string') {
    return JSON.stringify(value)
  }
  if (typeof value === 'number') {
    if (!Number.isSafeInteger(value) || Object.is(value, -0)) fail('INVALID_CANDIDATE')
    return String(value)
  }
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`
  if (!isPlainObject(value)) fail('INVALID_CANDIDATE')
  const keys = Object.keys(value)
  if (keys.some((key) => !/^[\x20-\x7e]+$/.test(key))) fail('INVALID_CANDIDATE')
  return `{${keys.sort().map((key) =>
    `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(',')}}`
}

function sha256(value) {
  return `sha256:${createHash('sha256').update(value).digest('hex')}`
}

function canonicalDigest(value) {
  return sha256(Buffer.from(canonicalJson(value), 'utf8'))
}

module.exports = { canonicalJson, sha256, canonicalDigest }
