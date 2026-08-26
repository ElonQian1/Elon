const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(
  __dirname,
  '../src/features/user-browser/localAiResponseRefreshFlight.ts',
)
const output = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: filename,
}).outputText
const compiled = new Module(filename, module)
compiled.filename = filename
compiled.paths = module.paths
compiled._compile(output, filename)

const { LocalAiResponseRefreshFlight } = compiled.exports
const flight = new LocalAiResponseRefreshFlight()

const first = flight.reset()
assert.equal(flight.claim(first), 'run')
assert.equal(flight.claim(first), 'queued')
assert.equal(flight.claim(first), 'queued', 'many watchdog ticks must collapse into one rerun')
assert.equal(flight.settle(first), 'rerun')
assert.equal(flight.claim(first), 'run')
assert.equal(flight.settle(first), 'idle')

const second = flight.reset()
assert.notEqual(second, first)
assert.equal(flight.claim(first), 'stale')
assert.equal(flight.claim(second), 'run')
const third = flight.reset()
assert.equal(flight.settle(second), 'stale', 'an old request must not settle a newer response')
assert.equal(flight.claim(third), 'run')
assert.equal(flight.settle(third), 'idle')

console.log('PASS local AI response refresh single-flight coalescing and generation isolation')
