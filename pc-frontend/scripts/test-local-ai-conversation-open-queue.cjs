const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(
  __dirname,
  '../src/features/user-browser/localAiConversationOpenQueue.ts',
)
const output = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: filename,
}).outputText
const compiled = new Module(filename, module)
compiled.filename = filename
compiled.paths = module.paths
compiled._compile(output, filename)

const { LocalAiConversationOpenQueue } = compiled.exports

const queue = new LocalAiConversationOpenQueue('chatgpt:owner-a')
assert.equal(queue.enqueue('first'), true)
assert.deepEqual(queue.take(), {
  conversationId: 'first',
  sessionIdentity: 'chatgpt:owner-a',
})
assert.equal(queue.enqueue('second'), false)
assert.equal(queue.enqueue('latest'), false)
assert.equal(queue.hasPending(), true)
assert.deepEqual(queue.take(), {
  conversationId: 'latest',
  sessionIdentity: 'chatgpt:owner-a',
})
assert.equal(queue.hasPending(), false)
queue.finish()
assert.equal(queue.enqueue('after-finish'), true)

process.stdout.write('PASS Win local AI conversation open queue\n')
