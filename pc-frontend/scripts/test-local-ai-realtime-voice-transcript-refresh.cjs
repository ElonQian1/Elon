const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(
  __dirname,
  '../src/features/user-browser/localAiRealtimeVoiceTranscriptRefresh.ts',
)
const output = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: filename,
}).outputText
const compiled = new Module(filename, module)
compiled.filename = filename
compiled.paths = module.paths
compiled._compile(output, filename)

const {
  LOCAL_AI_REALTIME_VOICE_TRANSCRIPT_REFRESH_GAPS_MS,
  LocalAiRealtimeVoiceTranscriptRefreshFlight,
} = compiled.exports

assert.deepEqual(LOCAL_AI_REALTIME_VOICE_TRANSCRIPT_REFRESH_GAPS_MS, [250, 750, 1_500])

const flight = new LocalAiRealtimeVoiceTranscriptRefreshFlight()
const first = flight.start()
assert.equal(first.started, true)
assert.deepEqual(flight.start(), { generation: first.generation, started: false })
assert.deepEqual(flight.claim(first.generation), {
  status: 'run', action: 'private_conversation',
})
assert.equal(flight.claim(first.generation).status, 'busy')
assert.deepEqual(flight.settle(first.generation), { status: 'wait', delayMs: 750 })
assert.deepEqual(flight.claim(first.generation), { status: 'run', action: 'snapshot' })
assert.deepEqual(flight.settle(first.generation), { status: 'wait', delayMs: 1_500 })
assert.deepEqual(flight.claim(first.generation), { status: 'run', action: 'snapshot' })
assert.deepEqual(flight.settle(first.generation), { status: 'done' })
assert.equal(flight.claim(first.generation).status, 'done')

const second = flight.start()
assert.equal(second.started, true)
assert.notEqual(second.generation, first.generation)
assert.equal(flight.claim(second.generation).status, 'run')
flight.cancel()
assert.equal(flight.settle(second.generation).status, 'stale')
assert.equal(flight.claim(second.generation).status, 'stale')

console.log('PASS realtime voice transcript refresh serialization and generation isolation')
