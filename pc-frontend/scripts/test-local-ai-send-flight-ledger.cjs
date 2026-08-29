const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(
  __dirname,
  '../src/features/user-browser/localAiSendFlightLedger.ts',
)
const output = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: filename,
}).outputText
const compiled = new Module(filename, module)
compiled.filename = filename
compiled.paths = module.paths
compiled._compile(output, filename)

const { LocalAiSendFlightLedger } = compiled.exports
const ledger = new LocalAiSendFlightLedger()

const first = ledger.begin('chatgpt:owner', 'optimistic-1')
assert.ok(first)
assert.equal(ledger.begin('chatgpt:owner', 'optimistic-2'), null,
  'a synchronous second click must not create another official send')
assert.deepEqual(ledger.current('chatgpt:owner', 'optimistic-1'), first)
assert.deepEqual(ledger.activeClaim(), first)
assert.equal(ledger.isCurrent(first), true)
assert.equal(ledger.settle('chatgpt:owner', 'optimistic-1'), true)
assert.equal(ledger.isCurrent(first), false)
assert.equal(ledger.isGenerationCurrent(first), true,
  'the owning finally block may still release its own busy presentation')

const second = ledger.begin('chatgpt:owner', 'optimistic-2')
assert.ok(second)
assert.notEqual(second.generation, first.generation)
assert.equal(ledger.isGenerationCurrent(first), false,
  'a late receipt from an older send must not mutate the newer send')

ledger.invalidate()
assert.equal(ledger.activeClaim(), null)
assert.equal(ledger.isCurrent(second), false)
assert.equal(ledger.isGenerationCurrent(second), false,
  'a new-conversation or provider boundary must invalidate in-flight callbacks')
assert.equal(ledger.current('chatgpt:owner', 'optimistic-2'), null)

const third = ledger.begin('google-ai-mode:owner', 'optimistic-3')
assert.ok(third)
assert.equal(ledger.settle('chatgpt:owner', 'optimistic-3'), false)
assert.equal(ledger.isCurrent(third), true, 'another session cannot settle this flight')

const controllerSource = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/useLocalAiWebChatController.ts',
), 'utf8')
assert.match(controllerSource, /sendFlightLedger\.begin\(requestedSessionIdentity, pending\.id\)/)
assert.match(controllerSource, /beginLocalNewConversation[\s\S]*?sendFlightLedger\.invalidate\(\)/)

const dispatcherSource = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/createLocalAiPreparedPromptDispatcher.ts',
), 'utf8')
assert.match(dispatcherSource, /activeSessionIdentity\.current === claim\.sessionIdentity/)
assert.match(dispatcherSource, /isCurrent: \(\) => ownsCurrentSession\(\) && ledger\.isCurrent\(claim\)/)

process.stdout.write('PASS local AI send single-owner ledger and generation isolation\n')
