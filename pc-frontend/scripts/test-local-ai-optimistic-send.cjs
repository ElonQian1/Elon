const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(__dirname, '../src/features/user-browser/localAiOptimisticSend.ts')
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
  beginOptimisticLocalAiSend,
  beginPendingLocalAiResponse,
  mergeOptimisticLocalAiMessages,
  pendingLocalAiSendObserved,
  pendingLocalAiResponseObserved,
} = compiled.exports

const initial = [message('u-1', 'user', '重复问题'), message('a-1', 'assistant', '旧回答')]
const first = beginOptimisticLocalAiSend(initial, [], '重复问题', 'pending-1')
const firstResponse = beginPendingLocalAiResponse(first)
assert.equal(first.baselineMatchingUserCount, 1)
assert.equal(firstResponse.id, 'pending-1:assistant')
assert.deepEqual(
  mergeOptimisticLocalAiMessages(initial, [first], [firstResponse]).map((item) => item.id),
  ['u-1', 'a-1', 'pending-1', 'pending-1:assistant'],
  'the user bubble and native thinking row must appear before the official page acknowledges the command',
)
assert.equal(pendingLocalAiSendObserved(initial, first), false)
assert.equal(pendingLocalAiResponseObserved(initial, firstResponse), false)

const afterFirstOfficialSend = [...initial, message('u-2', 'user', '重复问题')]
assert.equal(pendingLocalAiSendObserved(afterFirstOfficialSend, first), true)
assert.deepEqual(
  mergeOptimisticLocalAiMessages(afterFirstOfficialSend, [first], [firstResponse]).map((item) => item.id),
  ['u-1', 'a-1', 'u-2', 'pending-1:assistant'],
  'the user bubble must deduplicate while the native thinking row remains until an assistant turn arrives',
)

const officialCursor = [
  ...afterFirstOfficialSend,
  message('a-2', 'assistant', '\u258d'),
]
const normalizedCursor = mergeOptimisticLocalAiMessages(officialCursor, [first], [firstResponse])
assert.equal(normalizedCursor.at(-1).id, 'a-2')
assert.equal(normalizedCursor.at(-1).state, 'streaming')
assert.equal(normalizedCursor.at(-1).content.length, 0)
assert.equal(
  pendingLocalAiResponseObserved(officialCursor, firstResponse),
  false,
  'a cursor-only official placeholder is not a completed answer',
)

const officialStreaming = [
  ...afterFirstOfficialSend,
  { ...message('a-2', 'assistant', ''), state: 'streaming' },
]
assert.equal(pendingLocalAiResponseObserved(officialStreaming, firstResponse), true)
assert.deepEqual(
  mergeOptimisticLocalAiMessages(officialStreaming, [first], [firstResponse]).map((item) => item.id),
  ['u-1', 'a-1', 'u-2', 'a-2'],
  'the native placeholder must yield to the official streaming assistant turn',
)

const officialAnswer = [
  ...afterFirstOfficialSend,
  message('a-2', 'assistant', '这是正式回答'),
]
assert.equal(pendingLocalAiResponseObserved(officialAnswer, firstResponse), true)
assert.deepEqual(
  mergeOptimisticLocalAiMessages(officialAnswer, [first], [firstResponse]).map((item) => item.id),
  ['u-1', 'a-1', 'u-2', 'a-2'],
)

const second = beginOptimisticLocalAiSend(initial, [first], '重复问题', 'pending-2')
assert.equal(second.baselineMatchingUserCount, 2)
assert.equal(
  pendingLocalAiSendObserved(afterFirstOfficialSend, second),
  false,
  'repeated prompts must not be deduplicated against an earlier turn',
)

const blank = beginOptimisticLocalAiSend(initial, [], '   ', 'pending-empty')
assert.equal(blank, null)

process.stdout.write('PASS local AI optimistic send, thinking state, and snapshot deduplication\n')

function message(id, role, text) {
  return {
    id,
    role,
    state: 'completed',
    content: [{ type: 'text', text }],
  }
}
