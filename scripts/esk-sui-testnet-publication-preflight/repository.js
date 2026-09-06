'use strict'

const { lstatSync, readdirSync } = require('node:fs')
const path = require('node:path')
const { createHash } = require('node:crypto')
const { fail } = require('./contract')
const { sha256 } = require('./canonical')
const { readFixedRepositoryFile, resolveFixedPath } = require('./files')
const { parseStrictJson } = require('./strict-json')
const { MAX_REPOSITORY_FILE_BYTES } = require('./contract')

const APPROVED_BASELINE_COMMIT = 'aebbfc41b910887725179bca46ceb2b0d793458f'
const TOOLCHAIN_PATH = 'scripts/esk-sui-toolchain-ci/toolchain-v1.json'
const TOOLCHAIN_CONTRACT_SHA256 =
  'sha256:c7226dcb2e707a5c48d2f469b0b611296e5b65c1cc786dfd832b3b78e22065b6'

const FIXED = Object.freeze({
  sui_release: 'testnet-v1.79.0',
  sui_cli_version: 'sui 1.79.0-46f18562f1f5',
  sui_source_commit: '46f18562f1f5af2438d35828e8b62d5e0b972db7',
  sui_cli_sha256: 'sha256:d9b7ff7b4bb3cbbf3f327ddf5998b388773956ce30c897798b56a6c0db9fee7f',
  framework_source_archive_sha256:
    'sha256:9046fce263794cca6772c59aacd328706d42522dd7408c0dcb4a7f2613016afd',
  framework_content_digest:
    'sha256:017e6a38b5d976c87b710e02b39d26988691d101bf914a42e6509c82d62e027b',
  currency_package_input_digest:
    'sha256:6266890971f1c4ee6be82e9441959ebf0d4588e0483113a99ec61e66ea208665',
  currency_production_bytecode_digest:
    'sha256:314273ecd53a54793c8b70f35e4a1e853fdc7c6751c20dc0baf0628907b03ca7',
  participation_package_input_digest:
    'sha256:7ca8100f358f179b9d937a091373d4dbcdefd2cd6b78c0de223409e07f5f0038',
  participation_local_production_bytecode_digest:
    'sha256:fa691e2e7d7c1c347b8fd88a2dc9f3ca2590ee56813c0bb313ef2ea8d477d3ef',
})

const PACKAGE_FILES = Object.freeze({
  esk_currency: Object.freeze([
    'Move.lock', 'Move.toml', 'sources/esk.move', 'tests/esk_tests.move',
  ]),
  yilong_participation: Object.freeze([
    'Move.lock', 'Move.toml', 'sources/genesis_allocation.move',
    'sources/team_vesting.move', 'tests/genesis_allocation_tests.move',
    'tests/team_vesting_tests.move',
  ]),
})

const MAX_PACKAGE_ENTRIES = 4096
const MAX_PACKAGE_DEPTH = 32

function canonicalSourceBytes(bytes) {
  return Buffer.from(bytes.toString('utf8').replace(/\r\n?/g, '\n'), 'utf8')
}

function repositoryLstat(absolute) {
  try { return lstatSync(absolute, { bigint: true }) } catch { fail('REPOSITORY_DRIFT') }
}

function sameNode(left, right) {
  return left.ino === right.ino &&
    (left.dev === 0n || right.dev === 0n || left.dev === right.dev)
}

function sameDirectorySnapshot(left, right) {
  return sameNode(left, right) && left.mtimeNs === right.mtimeNs &&
    left.ctimeNs === right.ctimeNs
}

function assertOrdinaryDirectoryChain(repoRoot, relative) {
  const absolute = resolveFixedPath(repoRoot, relative)
  const root = path.resolve(repoRoot)
  const segments = relative.split('/')
  let current = root
  for (const segment of ['', ...segments]) {
    if (segment) current = path.join(current, segment)
    const stat = repositoryLstat(current)
    if (!stat.isDirectory() || stat.isSymbolicLink()) fail('REPOSITORY_DRIFT')
  }
  return absolute
}

function isTrackedPackageFile(relative) {
  const name = path.posix.basename(relative).toLowerCase()
  return name === 'move.toml' || name === 'move.lock' || name.endsWith('.move')
}

function walkPackageDirectory(absolute, segments, inventory, state) {
  if (segments.length > MAX_PACKAGE_DEPTH) fail('REPOSITORY_DRIFT')
  const initial = repositoryLstat(absolute)
  if (!initial.isDirectory() || initial.isSymbolicLink()) fail('REPOSITORY_DRIFT')

  let entries
  try { entries = readdirSync(absolute, { withFileTypes: true }) } catch { fail('REPOSITORY_DRIFT') }
  const opened = repositoryLstat(absolute)
  if (!opened.isDirectory() || opened.isSymbolicLink() ||
      !sameDirectorySnapshot(initial, opened)) fail('REPOSITORY_DRIFT')

  entries.sort((left, right) => left.name < right.name ? -1 : left.name > right.name ? 1 : 0)
  for (const entry of entries) {
    state.entries += 1
    if (state.entries > MAX_PACKAGE_ENTRIES || !entry.name || entry.name === '.' ||
        entry.name === '..' || entry.name.includes('/') || entry.name.includes('\\') ||
        entry.name.includes('\0')) fail('REPOSITORY_DRIFT')

    const childSegments = [...segments, entry.name]
    const relative = childSegments.join('/')
    const childAbsolute = path.join(absolute, entry.name)
    const stat = repositoryLstat(childAbsolute)
    if (entry.isSymbolicLink() || stat.isSymbolicLink()) fail('REPOSITORY_DRIFT')

    if (stat.isDirectory()) {
      if (entry.name.toLowerCase() === 'build') {
        if (segments.length === 0) continue
        fail('REPOSITORY_DRIFT')
      }
      walkPackageDirectory(childAbsolute, childSegments, inventory, state)
      continue
    }
    if (!stat.isFile()) fail('REPOSITORY_DRIFT')

    const lowerName = entry.name.toLowerCase()
    if ((lowerName === 'move.toml' || lowerName === 'move.lock') && segments.length !== 0) {
      fail('REPOSITORY_DRIFT')
    }
    if (isTrackedPackageFile(relative)) inventory.push(relative)
  }

  const finished = repositoryLstat(absolute)
  if (!finished.isDirectory() || finished.isSymbolicLink() ||
      !sameDirectorySnapshot(opened, finished)) fail('REPOSITORY_DRIFT')
}

function assertMoveInventory(repoRoot, packagePath, expectedFiles) {
  if (!Array.isArray(expectedFiles) || new Set(expectedFiles).size !== expectedFiles.length ||
      expectedFiles.some((relative) => typeof relative !== 'string' || relative.includes('\\') ||
        path.posix.isAbsolute(relative) || relative.split('/').some((segment) => !segment ||
          segment === '.' || segment === '..'))) fail('REPOSITORY_DRIFT')

  const packageAbsolute = assertOrdinaryDirectoryChain(repoRoot, packagePath)
  const actual = []
  walkPackageDirectory(packageAbsolute, [], actual, { entries: 0 })
  assertOrdinaryDirectoryChain(repoRoot, packagePath)

  const expected = [...expectedFiles].sort()
  actual.sort()
  if (actual.length !== expected.length ||
      actual.some((relative, index) => relative !== expected[index])) fail('REPOSITORY_DRIFT')
}

function packageInputDigest(repoRoot, packagePath, relativeFiles) {
  assertMoveInventory(repoRoot, packagePath, relativeFiles)
  const hash = createHash('sha256')
  for (const relative of [...relativeFiles].sort()) {
    const bytes = readFixedRepositoryFile(repoRoot, `${packagePath}/${relative}`)
    hash.update(relative).update('\0').update(canonicalSourceBytes(bytes)).update('\0')
  }
  return `sha256:${hash.digest('hex')}`
}

function packageById(config, id) {
  if (!Array.isArray(config.packages)) fail('REPOSITORY_DRIFT')
  const matches = config.packages.filter((item) => item && item.id === id)
  if (matches.length !== 1) fail('REPOSITORY_DRIFT')
  return matches[0]
}

function assertToolchain(config) {
  const currency = packageById(config, 'esk_currency')
  const participation = packageById(config, 'yilong_participation')
  const checks = [
    config.schema === 'yilong.esk.sui.reproducible_toolchain.v1',
    config.sui_source_commit === FIXED.sui_source_commit,
    config.cli && config.cli.release === FIXED.sui_release,
    config.cli && config.cli.version === FIXED.sui_cli_version,
    config.cli && config.cli.source_commit === FIXED.sui_source_commit,
    config.cli && config.cli.binary_sha256 === FIXED.sui_cli_sha256,
    config.framework &&
      config.framework.archive_sha256 === FIXED.framework_source_archive_sha256,
    config.framework &&
      config.framework.tracked_content_digest === FIXED.framework_content_digest,
    currency.package_input_digest === FIXED.currency_package_input_digest,
    currency.build_evidence &&
      currency.build_evidence.digest === FIXED.currency_production_bytecode_digest,
    participation.package_input_digest === FIXED.participation_package_input_digest,
    participation.build_evidence &&
      participation.build_evidence.digest === FIXED.participation_local_production_bytecode_digest,
  ]
  if (checks.some((value) => !value)) fail('REPOSITORY_DRIFT')
}

function loadAndVerifyRepository(repoRoot = path.resolve(__dirname, '..', '..')) {
  const toolchainBytes = readFixedRepositoryFile(repoRoot, TOOLCHAIN_PATH)
  if (sha256(canonicalSourceBytes(toolchainBytes)) !== TOOLCHAIN_CONTRACT_SHA256) {
    fail('REPOSITORY_DRIFT')
  }
  const config = parseStrictJson(toolchainBytes, MAX_REPOSITORY_FILE_BYTES)
  assertToolchain(config)
  const currencyInput = packageInputDigest(
    repoRoot, 'contracts/sui/esk_currency', PACKAGE_FILES.esk_currency)
  const participationInput = packageInputDigest(
    repoRoot, 'contracts/sui/yilong_participation', PACKAGE_FILES.yilong_participation)
  if (currencyInput !== FIXED.currency_package_input_digest ||
      participationInput !== FIXED.participation_package_input_digest) {
    fail('REPOSITORY_DRIFT')
  }
  return {
    baseline_commit: APPROVED_BASELINE_COMMIT,
    toolchain_contract_sha256: TOOLCHAIN_CONTRACT_SHA256,
    repository_sources_verified: true,
    currency_package_input_digest: currencyInput,
    participation_package_input_digest: participationInput,
  }
}

module.exports = {
  APPROVED_BASELINE_COMMIT, TOOLCHAIN_CONTRACT_SHA256, FIXED, PACKAGE_FILES,
  canonicalSourceBytes, packageInputDigest, loadAndVerifyRepository,
}
