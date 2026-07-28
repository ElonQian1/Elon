const assert = require('assert')
const childProcess = require('child_process')
const fs = require('fs')
const os = require('os')
const path = require('path')

const KiB = 1024
const checker = path.join(__dirname, 'check-pc-frontend-bundle-budget.js')

function deterministicNoise(size) {
  const buffer = Buffer.alloc(size)
  let state = 0x12345678
  for (let index = 0; index < size; index += 1) {
    state ^= state << 13
    state ^= state >>> 17
    state ^= state << 5
    buffer[index] = state & 0xff
  }
  return buffer
}

function writeFixture(root, asyncChunk) {
  const assets = path.join(root, 'assets')
  fs.mkdirSync(assets, { recursive: true })
  const files = {
    'app-fixture.js': 'app',
    'vendor-fixture.js': 'vendor',
    'store-fixture.js': 'store',
    'ConversationPage-fixture.js': 'conversation',
    'ConversationPage-fixture.css': '.conversation{}',
    'UiTunerPage-fixture.css': '.tuner{}',
  }
  for (const [name, content] of Object.entries(files)) {
    fs.writeFileSync(path.join(assets, name), content)
  }
  fs.writeFileSync(path.join(assets, 'UiTunerPage-fixture.js'), asyncChunk)
}

function runCase(asyncChunk) {
  const fixture = fs.mkdtempSync(path.join(os.tmpdir(), 'elon-bundle-budget-'))
  try {
    writeFixture(fixture, asyncChunk)
    return childProcess.spawnSync(
      process.execPath,
      [checker, '--dist', fixture],
      { encoding: 'utf8' },
    )
  } finally {
    fs.rmSync(fixture, { force: true, recursive: true })
  }
}

const normal = runCase('a'.repeat(470 * KiB))
assert.strictEqual(normal.status, 0, normal.stderr)
assert.match(normal.stdout, /PC_BUNDLE_BUDGET_CHECK=passed largest async js/)
assert.match(normal.stdout, /PC_BUNDLE_BUDGET=passed assets=7 warnings=0/)

const warning = runCase('a'.repeat(481 * KiB))
assert.strictEqual(warning.status, 0, warning.stderr)
assert.match(warning.stdout, /PC_BUNDLE_BUDGET_CHECK=warning largest async js/)
assert.match(warning.stdout, /warning: raw 481\.00 KiB > soft 480\.00 KiB/)
assert.match(warning.stdout, /PC_BUNDLE_BUDGET=passed assets=7 warnings=1/)

const rawFailure = runCase('a'.repeat(521 * KiB))
assert.strictEqual(rawFailure.status, 1)
assert.match(rawFailure.stdout, /PC_BUNDLE_BUDGET_CHECK=failed largest async js/)
assert.match(rawFailure.stdout, /failure: raw 521\.00 KiB > hard 520\.00 KiB/)
assert.match(rawFailure.stderr, /PC_BUNDLE_BUDGET=failed failures=1 warnings=0/)

const gzipFailure = runCase(deterministicNoise(200 * KiB))
assert.strictEqual(gzipFailure.status, 1)
assert.match(gzipFailure.stdout, /failure: gzip .* > hard 140\.00 KiB/)
assert.match(gzipFailure.stderr, /PC_BUNDLE_BUDGET=failed failures=1 warnings=0/)

console.log('PC_FRONTEND_BUNDLE_BUDGET_TEST=passed cases=4')
