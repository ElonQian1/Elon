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

const navigationQueue = new LocalAiConversationOpenQueue('chatgpt:owner-b')
assert.equal(navigationQueue.enqueue('/c/first', 'open_conversation'), true)
assert.equal(navigationQueue.enqueue('/c/first', 'open_conversation'), false)
assert.equal(navigationQueue.enqueue('/project/one', 'open_project'), false)
assert.deepEqual(navigationQueue.take(), {
  conversationId: '/project/one',
  sessionIdentity: 'chatgpt:owner-b',
  action: 'open_project',
})
assert.equal(navigationQueue.hasPending(), false)
navigationQueue.finish()

const switchedSessionQueue = new LocalAiConversationOpenQueue('chatgpt:owner-c')
assert.equal(switchedSessionQueue.hasPending(), false)
assert.notEqual(switchedSessionQueue, navigationQueue)

const controllerSource = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/useLocalAiWebChatController.ts',
), 'utf8')
const deferredGuard = controllerSource.indexOf('busyAction && isLocalAiDeferredConversationAction(action)')
const genericBusyGuard = controllerSource.indexOf("busyAction || (action === 'send_prompt'")
assert.ok(deferredGuard >= 0 && deferredGuard < genericBusyGuard)

const sidebarSource = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/AiWebChatSidebar.tsx',
), 'utf8')
const directorySection = sidebarSource.slice(sidebarSource.indexOf('function DirectorySection'))
assert.doesNotMatch(directorySection, /disabled=\{Boolean\(web\.controller\.busyAction\)\}/)

process.stdout.write('PASS Win local AI conversation open queue\n')
