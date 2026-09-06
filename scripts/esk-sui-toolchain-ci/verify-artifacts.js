const assert = require('node:assert/strict')
const crypto = require('node:crypto')
const fs = require('node:fs')
const path = require('node:path')

const ANSI = /\u001b\[[0-?]*[ -/]*[@-~]/g
const TEST_LINE = /^(?:INCLUDING DEPENDENCY [A-Za-z0-9_]+|BUILDING [A-Za-z0-9_]+|Running Move unit tests|\[ PASS\s+\] [A-Za-z0-9_:]+|Test result: OK\. Total tests: \d+; passed: \d+; failed: \d+)$/
const EXPLICIT_DEPENDENCY_NOTE = '[NOTE] Dependencies on Sui, MoveStdlib, Bridge, DeepBook, and SuiSystem are automatically added, but this feature is disabled for your package because you have explicitly included dependencies on Sui. Consider removing these dependencies from `Move.toml`.'

function sha256(bytes) {
  return `sha256:${crypto.createHash('sha256').update(bytes).digest('hex')}`
}

function canonicalBytes(bytes) {
  return Buffer.from(bytes.toString('utf8').replace(/\r\n?/g, '\n'), 'utf8')
}

function resolveContainedRelativePath(root, relative, label = 'path') {
  if (typeof relative !== 'string' || !relative || relative.includes('\\') ||
      path.posix.isAbsolute(relative) || path.win32.isAbsolute(relative)) {
    throw new Error(`${label} must be a forward-slash relative path`)
  }
  const segments = relative.split('/')
  if (segments.some((segment) => !segment || segment === '.' || segment === '..' ||
      !/^[A-Za-z0-9._-]+$/.test(segment))) {
    throw new Error(`${label} contains an invalid path segment`)
  }
  const parent = path.resolve(root)
  const resolved = path.resolve(parent, ...segments)
  const relation = path.relative(parent, resolved)
  if (!relation || relation.startsWith(`..${path.sep}`) || relation === '..' ||
      path.isAbsolute(relation)) {
    throw new Error(`${label} escaped its approved parent`)
  }
  return resolved
}

function canonicalTestReceipt(value) {
  const lines = String(value).replace(ANSI, '').replace(/\r\n?/g, '\n').split('\n')
  const accepted = []
  for (const line of lines) {
    const normalized = line.trimEnd()
    if (!normalized.trim()) continue
    if (normalized === EXPLICIT_DEPENDENCY_NOTE) continue
    if (!TEST_LINE.test(normalized)) throw new Error(`unexpected Move test output: ${normalized.slice(0, 160)}`)
    accepted.push(normalized)
  }
  if (!accepted.length) throw new Error('Move test output is empty')
  return `${accepted.join('\n')}\n`
}

function productionBundleDigest(modulePaths) {
  const files = [...modulePaths].sort((left, right) => {
    const leftName = path.basename(left, '.mv')
    const rightName = path.basename(right, '.mv')
    return leftName < rightName ? -1 : leftName > rightName ? 1 : 0
  })
  const hash = crypto.createHash('sha256')
  for (const file of files) {
    hash.update(path.basename(file, '.mv')).update('\0')
    hash.update(fs.readFileSync(file)).update('\0')
  }
  return `sha256:${hash.digest('hex')}`
}

function buildEvidenceDigest(kind, modulePaths) {
  if (kind === 'single_module_sha256_v1') {
    assert.equal(modulePaths.length, 1, 'single-module evidence must bind exactly one module')
    return sha256(fs.readFileSync(modulePaths[0]))
  }
  if (kind === 'production_bytecode_bundle_v1') {
    assert.ok(modulePaths.length > 0, 'bundle evidence must bind at least one module')
    return productionBundleDigest(modulePaths)
  }
  throw new Error(`unsupported build evidence kind: ${String(kind)}`)
}

function packageInputDigest(packageRoot) {
  const files = []
  const visit = (directory) => {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const absolute = path.join(directory, entry.name)
      if (entry.isDirectory() && entry.name !== 'build') visit(absolute)
      else if (entry.isFile() &&
          (entry.name === 'Move.toml' || entry.name === 'Move.lock' || entry.name.endsWith('.move'))) {
        files.push(absolute)
      }
    }
  }
  visit(packageRoot)
  const hash = crypto.createHash('sha256')
  for (const file of files.sort()) {
    hash.update(path.relative(packageRoot, file).replaceAll('\\', '/')).update('\0')
    hash.update(canonicalBytes(fs.readFileSync(file))).update('\0')
  }
  return `sha256:${hash.digest('hex')}`
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'))
}

function expectedModuleFiles(buildRoot, packageConfig) {
  const packageRoot = resolveContainedRelativePath(buildRoot, packageConfig.path, 'package path')
  const directory = path.join(packageRoot, 'build', packageConfig.id, 'bytecode_modules')
  assert.ok(fs.existsSync(directory), `missing bytecode directory: ${directory}`)
  const actualNames = fs.readdirSync(directory)
    .filter((name) => name.endsWith('.mv'))
    .map((name) => path.basename(name, '.mv'))
    .sort()
  const expectedNames = [...packageConfig.build_evidence.modules].sort()
  assert.deepEqual(actualNames, expectedNames, `${packageConfig.id} production module set changed`)
  return expectedNames.map((name) => path.join(directory, `${name}.mv`))
}

function verifyTestReceipt(repoRoot, packageConfig, outputPath) {
  const actual = canonicalTestReceipt(fs.readFileSync(outputPath, 'utf8'))
  const evidencePath = resolveContainedRelativePath(
    repoRoot, packageConfig.test_evidence.path, 'test evidence path')
  const expected = canonicalBytes(fs.readFileSync(evidencePath)).toString('utf8')
  assert.equal(actual, expected, `${packageConfig.id} runtime output differs from approved evidence`)
  assert.equal(sha256(Buffer.from(actual, 'utf8')), packageConfig.test_evidence.digest,
    `${packageConfig.id} runtime output digest changed`)
  const summary = `Test result: OK. Total tests: ${packageConfig.test_count}; passed: ${packageConfig.test_count}; failed: 0\n`
  assert.ok(actual.endsWith(summary), `${packageConfig.id} must pass exactly ${packageConfig.test_count} tests`)
}

function verifyPackage(repoRoot, packageConfig, outputPath, buildRoot = repoRoot) {
  const packageRoot = resolveContainedRelativePath(repoRoot, packageConfig.path, 'package path')
  assert.equal(packageInputDigest(packageRoot), packageConfig.package_input_digest,
    `${packageConfig.id} package input digest changed`)
  const modules = expectedModuleFiles(buildRoot, packageConfig)
  const buildDigest = buildEvidenceDigest(packageConfig.build_evidence.kind, modules)
  assert.equal(buildDigest, packageConfig.build_evidence.digest,
    `${packageConfig.id} production bytecode digest changed`)
  verifyTestReceipt(repoRoot, packageConfig, outputPath)
}

function parseArguments(argv) {
  const allowed = new Set(['--repo', '--build-root', '--currency-output', '--participation-output'])
  const result = {}
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index]
    const value = argv[index + 1]
    if (!allowed.has(key) || !value) throw new Error(`invalid argument: ${key || '<missing>'}`)
    if (Object.hasOwn(result, key)) throw new Error(`duplicate argument: ${key}`)
    result[key] = value
  }
  for (const key of allowed) if (!result[key]) throw new Error(`missing argument: ${key}`)
  return result
}

function run(argv) {
  const args = parseArguments(argv)
  const repoRoot = path.resolve(args['--repo'])
  const buildRoot = path.resolve(args['--build-root'])
  const config = readJson(path.join(repoRoot, 'scripts/esk-sui-toolchain-ci/toolchain-v1.json'))
  const byId = Object.fromEntries(config.packages.map((item) => [item.id, item]))
  assert.deepEqual(Object.keys(byId).sort(), ['esk_currency', 'yilong_participation'])
  const checks = [
    [byId.esk_currency, path.resolve(args['--currency-output'])],
    [byId.yilong_participation, path.resolve(args['--participation-output'])],
  ]
  const failures = []
  for (const [packageConfig, outputPath] of checks) {
    try { verifyPackage(repoRoot, packageConfig, outputPath, buildRoot) } catch (error) {
      failures.push(error instanceof Error ? error.message : String(error))
    }
  }
  if (failures.length) throw new Error(failures.join(' | '))
  process.stdout.write('ESK_SUI_ARTIFACTS=verified currency=3/3 participation=13/13\n')
}

module.exports = {
  buildEvidenceDigest,
  canonicalTestReceipt,
  packageInputDigest,
  productionBundleDigest,
  resolveContainedRelativePath,
  verifyPackage,
}

if (require.main === module) {
  try {
    run(process.argv.slice(2))
  } catch (error) {
    process.stderr.write(`ESK_SUI_ARTIFACTS=failed ${error instanceof Error ? error.message : String(error)}\n`)
    process.exitCode = 1
  }
}
