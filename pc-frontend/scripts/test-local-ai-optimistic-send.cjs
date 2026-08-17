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
  mergeOptimisticLocalAiMessages,
  pendingLocalAiSendObserved,
} = compiled.exports

const initial = [message('u-1', 'user', '重复问题'), message('a-1', 'assistant', '旧回答')]
const first = beginOptimisticLocalAiSend(initial, [], '重复问题', 'pending-1')
assert.equal(first.baselineMatchingUserCount, 1)
assert.deepEqual(
  mergeOptimisticLocalAiMessages(initial, [first]).map((item) => item.id),
  ['u-1', 'a-1', 'pending-1'],
  'the user bubble must appear before the official page acknowledges the command',
)
assert.equal(pendingLocalAiSendObserved(initial, first), false)

const afterFirstOfficialSend = [...initial, message('u-2', 'user', '重复问题')]
assert.equal(pendingLocalAiSendObserved(afterFirstOfficialSend, first), true)
assert.deepEqual(
  mergeOptimisticLocalAiMessages(afterFirstOfficialSend, [first]).map((item) => item.id),
  ['u-1', 'a-1', 'u-2'],
  'the optimistic bubble must disappear when the same official user turn arrives',
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

process.stdout.write('PASS local AI optimistic send and snapshot deduplication\n')

function message(id, role, text) {
  return {
    id,
    role,
    state: 'completed',
    content: [{ type: 'text', text }],
  }
}
