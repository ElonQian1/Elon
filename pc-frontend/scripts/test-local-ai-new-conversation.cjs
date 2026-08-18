const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(__dirname, '../src/features/user-browser/localAiNewConversation.ts')
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

const { selectLocalAiNewConversationPath } = compiled.exports

const liveSession = {
  windowStatus: 'ready',
  loading: false,
  rendererStatus: 'active',
  semanticCacheStatus: 'live',
  contextReady: true,
}
const liveSnapshot = { composerReady: true }

assert.equal(selectLocalAiNewConversationPath(null, null), 'home')
assert.equal(selectLocalAiNewConversationPath({
  ...liveSession,
  rendererStatus: 'connecting',
  semanticCacheStatus: 'cached',
  contextReady: false,
}, { composerReady: false }), 'home')
assert.equal(selectLocalAiNewConversationPath({
  ...liveSession,
  contextReady: false,
}, liveSnapshot), 'home')
assert.equal(selectLocalAiNewConversationPath({
  ...liveSession,
  semanticCacheStatus: 'cached',
}, liveSnapshot), 'home')
assert.equal(selectLocalAiNewConversationPath(liveSession, { composerReady: false }), 'home')
assert.equal(selectLocalAiNewConversationPath(liveSession, liveSnapshot), 'adapter')

process.stdout.write('PASS local AI new-conversation recovery policy\n')
