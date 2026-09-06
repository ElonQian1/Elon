'use strict'

const {
  closeSync, constants, fstatSync, lstatSync, openSync, readSync,
} = require('node:fs')
const path = require('node:path')
const {
  MAX_CANDIDATE_BYTES, MAX_REPOSITORY_FILE_BYTES, PreflightError, fail,
} = require('./contract')
const { parseStrictJson } = require('./strict-json')

function sameFile(left, right) {
  return left.ino === right.ino && (left.dev === 0n || right.dev === 0n || left.dev === right.dev)
}

function sameSnapshot(left, right) {
  return sameFile(left, right) && left.size === right.size &&
    left.mtimeNs === right.mtimeNs && left.ctimeNs === right.ctimeNs
}

function rejectNonLocalPath(input) {
  if (typeof input !== 'string' || input.length === 0 || input.length > 4096 || input.includes('\0')) {
    fail('INVALID_INPUT_PATH')
  }
  if (/^[\\/]{2}/.test(input) || /^(?:file|https?|smb):/i.test(input) ||
      /^(?:\\\\[?.]\\|\\[?.]\\)/.test(input)) fail('INVALID_INPUT_PATH')
  const firstColon = input.indexOf(':')
  if (firstColon !== -1) {
    const ordinaryDrivePrefix = firstColon === 1 && /^[A-Za-z]:[\\/]/.test(input)
    if (!ordinaryDrivePrefix || input.indexOf(':', firstColon + 1) !== -1) {
      fail('INVALID_INPUT_PATH')
    }
  }
  const segments = input.replaceAll('\\', '/').split('/')
  if (segments.some((segment) => /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\..*)?$/i.test(segment))) {
    fail('INVALID_INPUT_PATH')
  }
}

function assertNoSymbolicPath(absolute) {
  const parsed = path.parse(absolute)
  const segments = absolute.slice(parsed.root.length).split(path.sep).filter(Boolean)
  let current = parsed.root
  for (let index = 0; index < segments.length; index += 1) {
    current = path.join(current, segments[index])
    let stat
    try { stat = lstatSync(current, { bigint: true }) } catch { fail('INVALID_INPUT_PATH') }
    if (stat.isSymbolicLink()) fail('INPUT_NOT_REGULAR_FILE')
    if (index < segments.length - 1 && !stat.isDirectory()) fail('INVALID_INPUT_PATH')
  }
}

function readOrdinaryFile(input, maximumBytes, { allowEmpty = false } = {}) {
  rejectNonLocalPath(input)
  const absolute = path.resolve(input)
  assertNoSymbolicPath(absolute)
  let initial
  try {
    initial = lstatSync(absolute, { bigint: true })
  } catch (error) {
    if (error instanceof PreflightError) throw error
    fail('INVALID_INPUT_PATH')
  }
  if (!initial.isFile() || initial.isSymbolicLink() || (!allowEmpty && initial.size === 0n)) {
    fail('INPUT_NOT_REGULAR_FILE')
  }
  if (initial.size > BigInt(maximumBytes)) fail('INPUT_TOO_LARGE')

  let descriptor
  try {
    descriptor = openSync(absolute, constants.O_RDONLY | (constants.O_NOFOLLOW || 0))
    const opened = fstatSync(descriptor, { bigint: true })
    if (!opened.isFile() || !sameFile(initial, opened) || (!allowEmpty && opened.size === 0n)) {
      fail('INPUT_NOT_REGULAR_FILE')
    }
    if (opened.size > BigInt(maximumBytes)) fail('INPUT_TOO_LARGE')
    const buffer = Buffer.alloc(maximumBytes + 1)
    let length = 0
    while (length < buffer.length) {
      const count = readSync(descriptor, buffer, length, buffer.length - length, null)
      if (count === 0) break
      length += count
    }
    if (length > maximumBytes) fail('INPUT_TOO_LARGE')
    const finished = fstatSync(descriptor, { bigint: true })
    if (BigInt(length) !== opened.size || !sameSnapshot(opened, finished)) {
      fail('INPUT_NOT_REGULAR_FILE')
    }
    assertNoSymbolicPath(absolute)
    return buffer.subarray(0, length)
  } catch (error) {
    if (error instanceof PreflightError) throw error
    fail('INPUT_NOT_REGULAR_FILE')
  } finally {
    if (descriptor !== undefined) {
      try { closeSync(descriptor) } catch { /* error projection remains fixed */ }
    }
  }
}

function readCandidateFile(input) {
  return parseStrictJson(readOrdinaryFile(input, MAX_CANDIDATE_BYTES), MAX_CANDIDATE_BYTES)
}

function resolveFixedPath(repoRoot, relative) {
  if (typeof relative !== 'string' || relative.includes('\\') ||
      path.posix.isAbsolute(relative) || path.win32.isAbsolute(relative)) fail('REPOSITORY_DRIFT')
  const segments = relative.split('/')
  if (segments.some((segment) => !segment || segment === '.' || segment === '..' ||
      !/^[A-Za-z0-9._-]+$/.test(segment))) fail('REPOSITORY_DRIFT')
  const root = path.resolve(repoRoot)
  const result = path.resolve(root, ...segments)
  const relation = path.relative(root, result)
  if (!relation || relation === '..' || relation.startsWith(`..${path.sep}`) || path.isAbsolute(relation)) {
    fail('REPOSITORY_DRIFT')
  }
  return result
}

function readFixedRepositoryFile(repoRoot, relative) {
  try {
    return readOrdinaryFile(resolveFixedPath(repoRoot, relative), MAX_REPOSITORY_FILE_BYTES)
  } catch (error) {
    if (error instanceof PreflightError && error.code === 'INPUT_TOO_LARGE') throw error
    fail('REPOSITORY_DRIFT')
  }
}

function readFixedRepositoryJson(repoRoot, relative) {
  try {
    return parseStrictJson(readFixedRepositoryFile(repoRoot, relative), MAX_REPOSITORY_FILE_BYTES)
  } catch (error) {
    if (error instanceof PreflightError && error.code === 'INPUT_TOO_LARGE') throw error
    fail('REPOSITORY_DRIFT')
  }
}

module.exports = {
  readOrdinaryFile, readCandidateFile, resolveFixedPath,
  readFixedRepositoryFile, readFixedRepositoryJson,
}
