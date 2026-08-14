const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(__dirname, '../src/features/user-browser/localAiSnapshotCache.ts')
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
const { LocalAiSnapshotCache } = compiled.exports

const cache = new LocalAiSnapshotCache(2)
const chatgpt = state('chatgpt', 'chat-title')
cache.remember('chatgpt', 'owner-a', chatgpt)
assert.equal(cache.read('chatgpt', 'owner-a')?.semanticEvent?.title, 'chat-title')
assert.equal(cache.read('chatgpt', 'owner-b'), null)
assert.equal(cache.read('google-ai-mode', 'owner-a'), null)

cache.remember('google-ai-mode', 'owner-a', state('google-ai-mode', 'google-title'))
cache.read('chatgpt', 'owner-a')
cache.remember('chatgpt', 'owner-b', state('chatgpt', 'second-owner'))
assert.equal(cache.read('google-ai-mode', 'owner-a'), null, 'least recently used entry must be evicted')
assert.equal(cache.read('chatgpt', 'owner-a')?.semanticEvent?.title, 'chat-title')
assert.equal(cache.read('chatgpt', 'owner-b')?.semanticEvent?.title, 'second-owner')

cache.forget('chatgpt', 'owner-a')
assert.equal(cache.read('chatgpt', 'owner-a'), null)
assert.throws(
  () => cache.remember('chatgpt', 'owner-a', state('google-ai-mode', 'wrong-provider')),
  /provider/i,
)

cache.clear()
assert.equal(cache.size, 0)

process.stdout.write('PASS local AI owner/provider snapshot hot-cache contract\n')

function state(providerId, title) {
  return {
    providerId,
    windowLabel: `local-ai-${providerId}-test`,
    windowStatus: 'ready',
    windowVisible: false,
    currentUrl: '',
    currentHost: providerId === 'chatgpt' ? 'chatgpt.com' : 'google.com',
    loading: false,
    rendererStatus: 'active',
    cacheStatus: 'live',
    cacheUpdatedAtMs: 10,
    semanticEvent: {
      type: 'message_snapshot',
      title,
      url: '',
      draft: '',
      messages: [],
      authenticated: false,
      composerReady: true,
      streaming: false,
      currentModel: '',
      capabilities: [],
    },
    navigationEvent: null,
    commandResult: null,
    updatedAtMs: 10,
  }
}
