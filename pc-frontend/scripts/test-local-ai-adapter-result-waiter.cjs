const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(__dirname, '../src/features/user-browser/localAiAdapterResultWaiter.ts')
const source = fs.readFileSync(filename, 'utf8')
const output = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: filename,
}).outputText

function loadWaiter() {
  const compiled = new Module(filename, module)
  compiled.filename = filename
  compiled.paths = module.paths
  const defaultRequire = compiled.require.bind(compiled)
  compiled.require = (id) => {
    if (id === './localAiCommandReceipt') {
      return {
        findMatchingLocalAiCommandReceipt(latest, recent, action, requestId) {
          return [latest, ...(recent || [])]
            .find((item) => item?.action === action && item?.requestId === requestId)
        },
      }
    }
    if (id === './localAiAdapterTiming') {
      return { LOCAL_AI_RESULT_POLL_INTERVAL_MS: 200, localAiAdapterResultTimeoutMs: () => 12_000 }
    }
    if (id === './localAiNativeSessionUpdates') {
      return {
        listenLocalAiNativeSessionUpdates: async () => () => {},
        localAiNativeSessionUpdateMatches: (update, providerId, windowLabel) => (
          update.providerId === providerId && update.windowLabel === windowLabel
        ),
      }
    }
    return defaultRequire(id)
  }
  compiled._compile(output, filename)
  return compiled.exports
}

const { waitForLocalAiAdapterReceipts } = loadWaiter()
const request = { action: 'send_prompt', requestId: 'mcp_send_1' }
const pendingState = { windowLabel: 'local-ai-chatgpt-owner', commandResult: null, commandResults: [] }
const completedState = {
  ...pendingState,
  commandResult: { type: 'command_result', action: request.action, requestId: request.requestId, ok: true },
}

async function testImmediateReceipt() {
  let reads = 0
  let unlistened = false
  const result = await waitForLocalAiAdapterReceipts({
    providerId: 'chatgpt',
    requests: [request],
    readState: async () => { reads += 1; return completedState },
    listen: async () => () => { unlistened = true },
    pollIntervalMs: 1_000,
    timeoutMs: 500,
  })
  await new Promise((resolve) => setTimeout(resolve, 0))
  assert.equal(result, completedState)
  assert.equal(reads, 1)
  assert.equal(unlistened, true)
}

async function testNativeEventWakesBeforePoll() {
  let state = pendingState
  let handler
  let reads = 0
  const startedAt = Date.now()
  const waiting = waitForLocalAiAdapterReceipts({
    providerId: 'chatgpt',
    requests: [request],
    readState: async () => { reads += 1; return state },
    listen: async (next) => { handler = next; return () => {} },
    pollIntervalMs: 1_000,
    timeoutMs: 500,
  })
  await new Promise((resolve) => setTimeout(resolve, 20))
  state = completedState
  handler({ providerId: 'google_ai_mode', windowLabel: pendingState.windowLabel, kind: 'command_result' })
  handler({ providerId: 'chatgpt', windowLabel: 'wrong-window', kind: 'command_result' })
  handler({ providerId: 'chatgpt', windowLabel: pendingState.windowLabel, kind: 'message_snapshot' })
  await new Promise((resolve) => setTimeout(resolve, 20))
  assert.equal(reads, 1)
  handler({ providerId: 'chatgpt', windowLabel: pendingState.windowLabel, kind: 'command_result' })
  const result = await waiting
  assert.equal(result.commandResult.requestId, request.requestId)
  assert.equal(reads, 2)
  assert.ok(Date.now() - startedAt < 500)
}

async function testPollingFallback() {
  let reads = 0
  const result = await waitForLocalAiAdapterReceipts({
    providerId: 'chatgpt',
    requests: [request],
    readState: async () => { reads += 1; return reads === 1 ? pendingState : completedState },
    listen: async () => { throw new Error('events unavailable') },
    pollIntervalMs: 20,
    timeoutMs: 200,
  })
  assert.equal(result.commandResult.requestId, request.requestId)
  assert.equal(reads, 2)
}

async function testDeadline() {
  const result = await waitForLocalAiAdapterReceipts({
    providerId: 'chatgpt',
    requests: [request],
    readState: async () => pendingState,
    listen: async () => () => {},
    pollIntervalMs: 20,
    timeoutMs: 55,
  })
  assert.equal(result, null)
}

Promise.resolve()
  .then(testImmediateReceipt)
  .then(testNativeEventWakesBeforePoll)
  .then(testPollingFallback)
  .then(testDeadline)
  .then(() => process.stdout.write('PASS local AI native-event adapter result waiter\n'))
  .catch((error) => {
    console.error(error)
    process.exitCode = 1
  })
