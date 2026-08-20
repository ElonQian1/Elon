const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(__dirname, '../src/features/user-browser/localAiNewConversation.ts')
const source = fs.readFileSync(filename, 'utf8')
const controllerSource = fs.readFileSync(
  path.resolve(__dirname, '../src/features/user-browser/useLocalAiWebChatController.ts'),
  'utf8',
)
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

assert.equal(selectLocalAiNewConversationPath('chatgpt', null, null), 'home')
assert.equal(selectLocalAiNewConversationPath('chatgpt', {
  ...liveSession,
  rendererStatus: 'connecting',
  semanticCacheStatus: 'cached',
  contextReady: false,
}, { composerReady: false }), 'home')
assert.equal(selectLocalAiNewConversationPath('chatgpt', {
  ...liveSession,
  contextReady: false,
}, liveSnapshot), 'home')
assert.equal(selectLocalAiNewConversationPath('chatgpt', {
  ...liveSession,
  semanticCacheStatus: 'cached',
}, liveSnapshot), 'home')
assert.equal(selectLocalAiNewConversationPath('chatgpt', liveSession, { composerReady: false }), 'home')
assert.equal(selectLocalAiNewConversationPath('chatgpt', liveSession, liveSnapshot), 'adapter')
assert.equal(selectLocalAiNewConversationPath('google-ai-mode', liveSession, liveSnapshot), 'home')
assert.match(controllerSource, /GOOGLE_NEW_CONVERSATION_RELOAD_DELAY_MS/)
assert.match(controllerSource, /providerId !== 'google-ai-mode'/)
assert.match(controllerSource, /controlLocalAiWebSession\(providerId, ownerKey, 'reload'\)/)

process.stdout.write('PASS local AI new-conversation recovery policy\n')
