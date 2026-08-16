const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(__dirname, '../src/features/user-browser/localAiHistoryWindow.ts')
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
const { localAiHistoryWindow } = compiled.exports

assert.deepEqual(localAiHistoryWindow(null), {
  syncedCount: 0,
  observedCount: 0,
  windowStart: 0,
  complete: false,
  label: '当前会话历史待同步',
})

const complete = localAiHistoryWindow(snapshot(4, 4, 0))
assert.equal(complete.complete, true)
assert.match(complete.label, /完整同步/)

const partial = localAiHistoryWindow(snapshot(160, 248, 88))
assert.equal(partial.complete, false)
assert.equal(partial.observedCount, 248)
assert.match(partial.label, /160 \/ 官网观察 248/)

const repaired = localAiHistoryWindow(snapshot(6, 2, 4))
assert.equal(repaired.observedCount, 10)
assert.equal(repaired.windowStart, 4)

process.stdout.write('PASS local AI history window summary\n')

function snapshot(messageCount, observedMessageCount, messageWindowStart) {
  return {
    type: 'message_snapshot',
    title: '',
    url: '',
    draft: '',
    messages: Array.from({ length: messageCount }, (_, index) => ({
      id: `message-${index}`,
      role: index % 2 ? 'assistant' : 'user',
      state: 'completed',
      content: [],
    })),
    observedMessageCount,
    messageWindowStart,
    authenticated: false,
    composerReady: true,
    streaming: false,
    currentModel: '',
    capabilities: [],
  }
}
