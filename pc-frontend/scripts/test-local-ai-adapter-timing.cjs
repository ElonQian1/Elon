const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(__dirname, '../src/features/user-browser/localAiAdapterTiming.ts')
const source = fs.readFileSync(filename, 'utf8')
const output = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.CommonJS,
    target: ts.ScriptTarget.ES2020,
    esModuleInterop: true,
  },
  fileName: filename,
}).outputText
const compiled = new Module(filename, module)
compiled.filename = filename
compiled.paths = module.paths
compiled._compile(output, filename)

const {
  LOCAL_AI_RESULT_POLL_INTERVAL_MS,
  localAiAdapterResultAttempts,
  localAiAdapterResultTimeoutMs,
} = compiled.exports

assert.equal(LOCAL_AI_RESULT_POLL_INTERVAL_MS, 200)
assert.equal(localAiAdapterResultTimeoutMs('snapshot'), 5_000)
assert.equal(localAiAdapterResultTimeoutMs('send_prompt'), 12_000)
assert.equal(localAiAdapterResultTimeoutMs('list_conversations'), 12_000)
assert.equal(localAiAdapterResultAttempts('send_prompt'), 60)
assert.ok(localAiAdapterResultTimeoutMs('send_prompt') > 7_000)
assert.ok(localAiAdapterResultTimeoutMs('list_conversations') > 10_000)

process.stdout.write('PASS local AI action-specific adapter receipt timing\n')
