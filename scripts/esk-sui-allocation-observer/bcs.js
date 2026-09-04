const { createHash } = require('node:crypto')
const { AllocationObservationError } = require('./contract')

const MAX_U32 = 0xffff_ffffn

class BcsDecodeError extends AllocationObservationError {
  constructor() {
    super('BCS_MISMATCH')
  }
}

function fail() {
  throw new BcsDecodeError()
}

/** Decode only canonical, padded RFC 4648 Base64. */
function strictBase64(value) {
  if (typeof value !== 'string' || value.length % 4 !== 0 ||
    !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value)) fail()
  const bytes = Buffer.from(value, 'base64')
  if (bytes.toString('base64') !== value) fail()
  return bytes
}

function encodeUleb(value) {
  const bytes = []
  do {
    let byte = Number(value & 0x7fn)
    value >>= 7n
    if (value !== 0n) byte |= 0x80
    bytes.push(byte)
  } while (value !== 0n)
  return Buffer.from(bytes)
}

class Reader {
  constructor(bytes) {
    this.bytes = bytes
    this.offset = 0
  }

  take(length) {
    if (!Number.isSafeInteger(length) || length < 0 || this.offset + length > this.bytes.length) fail()
    const result = this.bytes.subarray(this.offset, this.offset + length)
    this.offset += length
    return result
  }

  uleb32() {
    const start = this.offset
    let value = 0n
    for (let index = 0; index < 5; index += 1) {
      const byte = this.take(1)[0]
      if (index === 4 && (byte & 0xf0) !== 0) fail()
      value |= BigInt(byte & 0x7f) << BigInt(index * 7)
      if ((byte & 0x80) === 0) {
        if (value > MAX_U32 || !this.bytes.subarray(start, this.offset).equals(encodeUleb(value))) fail()
        return Number(value)
      }
    }
    fail()
  }

  u64() {
    return this.take(8).readBigUInt64LE().toString()
  }

  address() {
    return `0x${this.take(32).toString('hex')}`
  }

  digest() {
    const length = this.uleb32()
    if (length !== 32) fail()
    return `sha256:${this.take(length).toString('hex')}`
  }

  finish() {
    if (this.offset !== this.bytes.length) fail()
  }
}

function decode(value, read) {
  const bytes = strictBase64(value)
  const reader = new Reader(bytes)
  const result = read(reader)
  reader.finish()
  return {
    ...result,
    bcs_sha256: `sha256:${createHash('sha256').update(bytes).digest('hex')}`,
  }
}

function decodeReceipt(value) {
  return decode(value, reader => ({
    id: reader.address(),
    manifest_digest: reader.digest(),
    total_base_units: reader.u64(),
    distribution: reader.address(),
    team_beneficiary: reader.address(),
    treasury: reader.address(),
    liquidity_recipient: reader.address(),
    user_migration_and_ecosystem_units: reader.u64(),
    team_vesting_units: reader.u64(),
    project_treasury_units: reader.u64(),
    liquidity_units: reader.u64(),
    community_contributors_units: reader.u64(),
    security_operations_reserve_units: reader.u64(),
    start_ms: reader.u64(),
    cliff_ms: reader.u64(),
    end_ms: reader.u64(),
    executed_at_ms: reader.u64(),
    user_migration_and_ecosystem_coin_id: reader.address(),
    team_vesting_id: reader.address(),
    project_treasury_coin_id: reader.address(),
    liquidity_coin_id: reader.address(),
    community_contributors_coin_id: reader.address(),
    security_operations_reserve_coin_id: reader.address(),
  }))
}

function decodeVesting(value) {
  return decode(value, reader => ({
    id: reader.address(),
    beneficiary: reader.address(),
    total_base_units: reader.u64(),
    claimed_base_units: reader.u64(),
    start_ms: reader.u64(),
    cliff_ms: reader.u64(),
    end_ms: reader.u64(),
    remaining_base_units: reader.u64(),
  }))
}

function decodeCoin(value) {
  return decode(value, reader => ({ id: reader.address(), balance: reader.u64() }))
}

function decodeCap(value) {
  return decode(value, reader => ({ id: reader.address() }))
}

module.exports = {
  BcsDecodeError, strictBase64, decodeReceipt, decodeVesting, decodeCoin, decodeCap,
}
