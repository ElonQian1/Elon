const {
  closeSync, constants, fstatSync, lstatSync, openSync, readSync,
} = require('node:fs')
const { resolve } = require('node:path')
const { MAX_FILE_BYTES, fail, safeCode } = require('./contract')
const { createChallenge } = require('./challenge')
const { verifyAddressControl } = require('./verify')
const { parseUniqueJson } = require('./json')

const HELP = [
  'Offline Sui personal-message address-control proof; no wallet, RPC or transaction execution.',
  'This tool does not bind a platform account, consume a challenge or enable an ESK balance.',
  'node scripts/prepare-esk-sui-address-binding.js challenge <request.json>',
  'node scripts/prepare-esk-sui-address-binding.js verify <challenge.json> <wallet-response.json>',
].join('\n')

function localPath(path) {
  if (typeof path !== 'string' || path.length === 0 || path.length > 4096) fail('INVALID_INPUT')
  if (/^[\\/]{2}/.test(path)) fail('INVALID_INPUT')
  const absolute = resolve(path)
  if (/^[\\/]{2}/.test(absolute)) fail('INVALID_INPUT')
  return absolute
}

function sameFile(left, right) {
  if (left.ino !== right.ino) return false
  return left.dev === 0n || right.dev === 0n || left.dev === right.dev
}

function sameSnapshot(left, right) {
  return sameFile(left, right) && left.size === right.size &&
    left.mtimeNs === right.mtimeNs && left.ctimeNs === right.ctimeNs
}

function readLocalFile(path) {
  const absolute = localPath(path)
  let initial
  try { initial = lstatSync(absolute, { bigint: true }) } catch { fail('INVALID_INPUT') }
  if (!initial.isFile() || initial.isSymbolicLink() || initial.size <= 0n) fail('INVALID_INPUT')
  if (initial.size > BigInt(MAX_FILE_BYTES)) fail('FILE_TOO_LARGE')
  let descriptor
  try {
    descriptor = openSync(absolute, constants.O_RDONLY)
    const opened = fstatSync(descriptor, { bigint: true })
    if (!opened.isFile() || !sameFile(initial, opened) || opened.size <= 0n) fail('INVALID_INPUT')
    if (opened.size > BigInt(MAX_FILE_BYTES)) fail('FILE_TOO_LARGE')
    const buffer = Buffer.alloc(MAX_FILE_BYTES + 1)
    let length = 0
    while (length < buffer.length) {
      const count = readSync(descriptor, buffer, length, buffer.length - length, null)
      if (count === 0) break
      length += count
    }
    if (length > MAX_FILE_BYTES) fail('FILE_TOO_LARGE')
    const finished = fstatSync(descriptor, { bigint: true })
    if (BigInt(length) !== opened.size || !sameSnapshot(opened, finished)) fail('INVALID_INPUT')
    return buffer.subarray(0, length)
  } catch (error) {
    if (error && error.name === 'AddressBindingError') throw error
    fail('INVALID_INPUT')
  } finally {
    if (descriptor !== undefined) {
      try { closeSync(descriptor) } catch { /* fixed error projection happens above */ }
    }
  }
}

function readJson(path) {
  let value
  try {
    const bytes = readLocalFile(path)
    if (bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf) {
      fail('INVALID_INPUT')
    }
    value = parseUniqueJson(bytes.toString('utf8'))
  } catch (error) {
    if (error && error.name === 'AddressBindingError') throw error
    fail('INVALID_INPUT')
  }
  return value
}

async function run(args) {
  if (args.length === 1 && args[0] === '--help') return { help: HELP }
  if (args.length === 2 && args[0] === 'challenge') return createChallenge(readJson(args[1]))
  if (args.length === 3 && args[0] === 'verify') {
    return verifyAddressControl(readJson(args[1]), readJson(args[2]))
  }
  fail('USAGE')
}

async function main(args, io = { out: console.log, error: console.error }) {
  try {
    const result = await run(args)
    if (result.help) io.out(result.help)
    else io.out(JSON.stringify(result, null, 2))
    return 0
  } catch (error) {
    io.error(`ESK_SUI_ADDRESS_BINDING_ERROR=${safeCode(error)}`)
    return 1
  }
}

module.exports = { HELP, localPath, readLocalFile, readJson, run, main }
